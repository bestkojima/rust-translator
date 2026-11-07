use serde_json::Value;

// SSE is line-based, not HTTP-chunk based: UTF-8 characters, CRLF and even
// the blank line ending an event may be split over arbitrary network reads.
#[derive(Default)]
struct SseDecoder {
    bytes: Vec<u8>,
    data: Vec<String>,
    event: String,
    first_line: bool,
}

impl SseDecoder {
    fn new() -> Self { Self { first_line: true, ..Self::default() } }

    fn push(&mut self, bytes: &[u8]) -> Result<Vec<(String, String)>, String> {
        self.bytes.extend_from_slice(bytes);
        let mut events = Vec::new();
        loop {
            let Some(end) = self.bytes.iter().position(|b| *b == b'\n' || *b == b'\r') else { break };
            // A trailing CR might be the first half of CRLF in the next read.
            if self.bytes[end] == b'\r' && end + 1 == self.bytes.len() { break; }
            let separator = if self.bytes[end] == b'\r' && self.bytes.get(end + 1) == Some(&b'\n') { 2 } else { 1 };
            let line = self.bytes.drain(..end + separator).collect::<Vec<_>>();
            let mut line = std::str::from_utf8(&line[..end]).map_err(|_| "流式响应包含无效 UTF-8")?;
            if self.first_line { line = line.strip_prefix('\u{feff}').unwrap_or(line); self.first_line = false; }
            if line.is_empty() {
                if !self.data.is_empty() { events.push((std::mem::take(&mut self.event), self.data.join("\n"))); self.data.clear(); }
                else { self.event.clear(); }
            } else if !line.starts_with(':') {
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                let value = value.strip_prefix(' ').unwrap_or(value);
                match field { "event" => self.event = value.to_owned(), "data" => self.data.push(value.to_owned()), _ => {} }
            }
        }
        if self.bytes.len() > 1024 * 1024 { return Err("流式响应数据过大".into()); }
        Ok(events)
    }
}

#[derive(Default)]
struct StreamState { text: String, finished: bool }

impl StreamState {
    fn event<F: FnMut(&str) -> Result<(), String>>(&mut self, provider: &str, event: &str, data: &str, emit: &mut F) -> Result<(), String> {
        if data == "[DONE]" { self.finished = true; return Ok(()); }
        let body: Value = serde_json::from_str(data).map_err(|_| "流式响应格式无法识别")?;
        if event == "error" || body.get("error").is_some() || body.get("type").and_then(Value::as_str) == Some("error") {
            return Err("翻译服务在流式输出时返回错误，请检查模型权限或重试".into());
        }
        let mut delta = None;
        let mut reason = None;
        if provider == "anthropic" {
            match body.get("type").and_then(Value::as_str) {
                Some("content_block_delta") if body.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") => delta = body.pointer("/delta/text").and_then(Value::as_str),
                Some("message_delta") => reason = body.pointer("/delta/stop_reason").and_then(Value::as_str),
                Some("message_stop") => self.finished = true,
                _ => {}
            }
        } else if let Some(choice) = body.pointer("/choices/0") {
            delta = choice.pointer("/delta/content").and_then(Value::as_str);
            reason = choice.get("finish_reason").and_then(Value::as_str);
        }
        if let Some(text) = delta.filter(|s| !s.is_empty()) { self.text.push_str(text); emit(text)?; }
        match reason {
            Some("length" | "max_tokens") => return Err("译文被长度限制截断，请缩短原文后重试".into()),
            Some("content_filter") => return Err("翻译服务过滤了该内容，无法完成翻译".into()),
            Some("tool_calls" | "function_call") => return Err("模型返回了工具调用，请选择支持直接文本输出的模型".into()),
            Some("stop") if provider != "anthropic" => self.finished = true,
            Some("end_turn" | "stop_sequence") if provider == "anthropic" => {},
            Some(_) => return Err("模型未正常完成翻译，请检查模型响应或重试".into()),
            None => {}
        }
        Ok(())
    }
}

pub async fn translate<F: FnMut(&str) -> Result<(), String>>(
    client: &reqwest::Client, provider: &str, endpoint: &str, key: &str, payload: &Value, mut emit: F,
) -> Result<String, String> {
    let request = client.post(endpoint);
    let request = if provider == "anthropic" { request.header("x-api-key", key).header("anthropic-version", "2023-06-01") } else { request.bearer_auth(key) };
    let mut response = request.json(payload).send().await.map_err(|e| if e.is_timeout() { "翻译服务请求超时，请检查网络后重试" } else { "无法连接翻译服务，请检查网络或代理" }.to_string())?;
    let status = response.status();
    if !status.is_success() { return Err(match status.as_u16() {
        300..=399 => format!("翻译服务返回重定向（HTTP {status}），为保护密钥未跟随"),
        400 => "翻译服务拒绝了请求（HTTP 400），请检查模型及请求参数".into(),
        401 => "翻译服务认证失败（HTTP 401），请检查 API Key".into(),
        403 => "翻译服务拒绝访问（HTTP 403），请检查模型权限和账户额度".into(),
        404 => "翻译服务未找到接口或模型（HTTP 404），请核对模型是否可用".into(),
        429 => "翻译服务请求过快或额度不足（HTTP 429），请查看控制台".into(),
        _ => format!("翻译服务返回错误（HTTP {status}）"),
    }); }
    let mut decoder = SseDecoder::new();
    let mut state = StreamState::default();
    while let Some(bytes) = response.chunk().await.map_err(|e| if e.is_timeout() { "翻译服务请求超时" } else { "流式连接中断，请重试" }.to_string())? {
        for (event, data) in decoder.push(&bytes)? { state.event(provider, &event, &data, &mut emit)?; }
    }
    if decoder.bytes.last() == Some(&b'\r') {
        for (event, data) in decoder.push(b"\n")? { state.event(provider, &event, &data, &mut emit)?; }
    }
    if !state.finished { return Err("流式连接提前结束，译文可能不完整，请重试".into()); }
    if state.text.trim().is_empty() { return Err("翻译服务没有返回译文，请检查模型权限或重试".into()); }
    Ok(state.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, time::Duration};

    #[test]
    fn utf8_and_crlf_split_across_every_byte() {
        let raw = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"},\"finish_reason\":null}]}\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"世界\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: [DONE]\r\n\r\n";
        let mut decoder = SseDecoder::new(); let mut state = StreamState::default(); let mut deltas = Vec::new();
        for b in raw.as_bytes() { for (event, data) in decoder.push(&[*b]).unwrap() { state.event("deepseek", &event, &data, &mut |s| { deltas.push(s.to_owned()); Ok(()) }).unwrap(); } }
        assert_eq!(deltas, ["你好", "世界"]); assert_eq!(state.text, "你好世界"); assert!(state.finished);
    }

    #[test]
    fn anthropic_ignores_thinking_and_reports_errors_and_truncation() {
        let mut state = StreamState::default(); let mut output = String::new(); let mut emit = |s: &str| { output.push_str(s); Ok(()) };
        state.event("anthropic", "", r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"private"}}"#, &mut emit).unwrap();
        state.event("anthropic", "", r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"你好"}}"#, &mut emit).unwrap();
        assert!(state.event("anthropic", "", r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#, &mut emit).is_err());
        assert!(state.event("anthropic", "error", r#"{"type":"error","error":{"message":"internal"}}"#, &mut emit).is_err());
        assert_eq!(output, "你好");
    }

    #[test]
    fn http_sse_is_delivered_incrementally() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/chat", listener.local_addr().unwrap());
        let (first_tx, first_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = vec![0; 4096]; let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]);
            assert!(request.to_lowercase().contains("authorization: bearer test-key"));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
            let first = "data: {\"choices\":[{\"delta\":{\"content\":\"你\"},\"finish_reason\":null}]}\n\n";
            write!(stream, "{:X}\r\n{}\r\n", first.len(), first).unwrap(); stream.flush().unwrap();
            // The client must receive the first piece while the connection is still open.
            first_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let second = "data: {\"choices\":[{\"delta\":{\"content\":\"好\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            write!(stream, "{:X}\r\n{}\r\n0\r\n\r\n", second.len(), second).unwrap();
        });
        let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(6)).build().unwrap();
        let mut received = Vec::new();
        let output = tauri::async_runtime::block_on(translate(&client, "deepseek", &endpoint, "test-key", &serde_json::json!({"stream":true}), |s| {
            received.push(s.to_owned()); if received.len() == 1 { first_tx.send(()).unwrap(); } Ok(())
        })).unwrap();
        assert_eq!(received, ["你", "好"]); assert_eq!(output, "你好"); server.join().unwrap();
    }
}
