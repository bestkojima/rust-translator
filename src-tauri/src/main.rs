#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::{Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
use tauri::{Emitter, Manager, PhysicalPosition, Position, State, WebviewUrl, WebviewWindowBuilder};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

const MODEL: &str = "tencent/Hunyuan-MT-7B";
const SERVICE: &str = "com.wordglance.app";

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    enabled: bool,
    excluded_apps: Vec<String>,
    target_language: String,
    // A pre-existing configuration without this field used SiliconFlow.
    #[serde(default = "legacy_provider")]
    provider: String,
    #[serde(default = "default_source_language")]
    source_language: String,
}

fn legacy_provider() -> String { "siliconflow".into() }
fn default_source_language() -> String { "en".into() }

impl Default for Settings {
    fn default() -> Self {
        Self { enabled: true, excluded_apps: vec!["WindowsTerminal.exe".into(), "cmd.exe".into(), "powershell.exe".into(), "pwsh.exe".into(), "1Password.exe".into(), "KeePass.exe".into()], target_language: "zh".into(), provider: "siliconflow".into(), source_language: default_source_language() }
    }
}

#[derive(Serialize)]
struct SettingsResponse {
    enabled: bool,
    excluded_apps: Vec<String>,
    target_language: String,
    provider: String,
    source_language: String,
    has_key: bool,
    credential_error: Option<String>,
    capture_error: Option<String>,
}

struct AppState {
    settings: Mutex<Settings>,
    pending: Mutex<String>,
    credential_lock: Mutex<()>,
    capture_error: Mutex<Option<String>>,
    translation_ready: AtomicBool,
    translation_pending: AtomicBool,
    client: reqwest::Client,
    config_path: PathBuf,
}

fn key_entry() -> Result<keyring_core::Entry, String> {
    let modifiers = HashMap::from([("persistence", "Local")]);
    keyring_core::Entry::new_with_modifiers(SERVICE, "siliconflow", &modifiers).map_err(|e| format!("凭据存储不可用：{e}"))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, String> {
    let settings = state.settings.lock().map_err(|_| "读取设置失败")?.clone();
    let _guard = state.credential_lock.lock().map_err(|_| "读取凭据失败")?;
    let (has_key, credential_error) = match key_entry().and_then(|entry| {
        match entry.get_password() {
            Ok(value) => Ok(!value.is_empty()),
            Err(keyring_core::Error::NoEntry) => Ok(false),
            Err(e) => Err(format!("读取凭据失败：{e}")),
        }
    }) {
        Ok(has_key) => (has_key, None),
        Err(error) => (false, Some(error)),
    };
    let capture_error = state.capture_error.lock().map_err(|_| "读取划词状态失败")?.clone();
    Ok(SettingsResponse { enabled: settings.enabled, excluded_apps: settings.excluded_apps, target_language: settings.target_language, provider: settings.provider, source_language: settings.source_language, has_key, credential_error, capture_error })
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    if !["zh", "en", "ja"].contains(&settings.target_language.as_str()) { return Err("不支持该目标语言".into()); }
    if !["zh", "en", "ja"].contains(&settings.source_language.as_str()) { return Err("不支持该源语言".into()); }
    if !["mymemory", "siliconflow"].contains(&settings.provider.as_str()) { return Err("不支持该翻译服务".into()); }
    if let Some(dir) = state.config_path.parent() { fs::create_dir_all(dir).map_err(|e| format!("创建设置目录失败：{e}"))?; }
    let data = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&state.config_path, data).map_err(|e| format!("保存设置失败：{e}"))?;
    capture::set_enabled(settings.enabled);
    capture::set_excluded_apps(settings.excluded_apps.clone());
    if !settings.enabled { if let Some(window) = app.get_webview_window("overlay") { let _ = window.hide(); } }
    *state.settings.lock().map_err(|_| "保存设置失败")? = settings;
    Ok(())
}

#[tauri::command]
fn set_api_key(state: State<'_, AppState>, api_key: String) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() { return Err("请先填写 API Key".into()); }
    let _guard = state.credential_lock.lock().map_err(|_| "保存凭据失败")?;
    key_entry()?.set_password(key).map_err(|e| format!("保存凭据失败：{e}"))
}

#[tauri::command]
fn clear_api_key(state: State<'_, AppState>) -> Result<(), String> {
    let _guard = state.credential_lock.lock().map_err(|_| "删除凭据失败")?;
    match key_entry()?.delete_credential() { Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()), Err(e) => Err(format!("删除凭据失败：{e}")) }
}

#[tauri::command]
async fn translate_text(state: State<'_, AppState>, text: String) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() { return Err("请先选择或输入文字".into()); }
    if text.chars().count() > 5000 { return Err("单次最多翻译 5000 个字符".into()); }
    let settings = state.settings.lock().map_err(|_| "读取设置失败")?.clone();
    if settings.provider == "mymemory" {
        return translate_mymemory(&state.client, text, &settings.source_language, &settings.target_language).await;
    }
    let key = {
        let _guard = state.credential_lock.lock().map_err(|_| "读取凭据失败")?;
        key_entry()?.get_password().map_err(|_| "请先在设置中配置硅基流动 API Key")?
    };
    let language = match settings.target_language.as_str() { "en" => "英文", "ja" => "日文", _ => "中文" };
    let prompt = format!("把下面的文本翻译成{language}，不要额外解释。\n\n{text}");
    let response = state.client.post("https://api.siliconflow.cn/v1/chat/completions")
        .bearer_auth(key)
        .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":prompt}],"stream":false,"temperature":0.7,"top_p":0.6,"top_k":20,"max_tokens":2048}))
        .send().await.map_err(|e| if e.is_timeout() { "翻译超时，请检查网络后重试".to_string() } else { "无法连接翻译服务，请检查网络".to_string() })?;
    let status = response.status();
    if !status.is_success() { return Err(match status.as_u16() { 401 | 403 => "API Key 无效或账号没有调用权限".into(), 429 => "请求过于频繁，请稍后再试".into(), _ => format!("翻译服务返回错误（HTTP {status}）") }); }
    let body: serde_json::Value = response.json().await.map_err(|_| "翻译服务返回了无法识别的数据")?;
    let result = body.pointer("/choices/0/message/content").and_then(|v| v.as_str()).unwrap_or("").trim();
    if result.is_empty() { return Err("翻译服务没有返回译文".into()); }
    Ok(result.to_string())
}

async fn translate_mymemory(client: &reqwest::Client, text: &str, source: &str, target: &str) -> Result<String, String> {
    translate_mymemory_at(client, "https://api.mymemory.translated.net/get", text, source, target).await
}

async fn translate_mymemory_at(client: &reqwest::Client, endpoint: &str, text: &str, source: &str, target: &str) -> Result<String, String> {
    if text.len() > 500 { return Err("MyMemory 每次最多 500 个 UTF-8 字节，请缩短选中文字或切换硅基流动".into()); }
    if text.split('\n').any(|line| line.trim().is_empty()) { return Err("MyMemory 每次仅支持一个段落，请缩短选中文字或切换硅基流动".into()); }
    if source == target { return Err("MyMemory 的源语言与目标语言相同，请在偏好设置中调整".into()); }
    let to_code = |lang: &str| match lang { "zh" => "zh-CN", "ja" => "ja", _ => "en" };
    let pair = format!("{}|{}", to_code(source), to_code(target));
    let response = client.get(endpoint)
        .query(&[("q", text), ("langpair", pair.as_str())])
        .send().await.map_err(|e| if e.is_timeout() { "MyMemory 请求超时，请检查网络后重试".to_string() } else { "无法连接 MyMemory，请检查网络".to_string() })?;
    let status = response.status();
    if status.as_u16() == 429 { return Err("MyMemory 当前免费额度已用完或请求过快，请稍后再试，或在设置中切换硅基流动".into()); }
    if !status.is_success() { return Err(format!("MyMemory 服务返回错误（HTTP {status}）")); }
    let body: serde_json::Value = response.json().await.map_err(|_| "MyMemory 返回了无法识别的数据")?;
    let response_status = body.get("responseStatus").and_then(|s| s.as_u64());
    if response_status == Some(429) { return Err("MyMemory 当前免费额度已用完或请求过快，请稍后再试，或在设置中切换硅基流动".into()); }
    if response_status != Some(200) { return Err("MyMemory 无法完成翻译，请检查语言选择或稍后重试".into()); }
    let result = body.pointer("/responseData/translatedText").and_then(|s| s.as_str()).unwrap_or("").trim();
    if result.is_empty() { return Err("MyMemory 没有返回译文".into()); }
    Ok(html_escape::decode_html_entities(result).into_owned())
}

#[cfg(test)]
mod translation_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn old_settings_keep_siliconflow() {
        let old = r#"{"enabled":true,"excluded_apps":[],"target_language":"zh"}"#;
        let settings: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(settings.provider, "siliconflow");
        assert_eq!(settings.source_language, "en");
        assert_eq!(Settings::default().provider, "siliconflow");
    }

    #[test]
    fn mymemory_handles_response_and_quota_error() {
        fn serve(status: &str, body: &str) -> String {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/get", listener.local_addr().unwrap());
            let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 2048];
                let count = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..count]);
                assert!(request.starts_with("GET /get?"));
                assert!(request.contains("langpair=en%7Czh-CN"));
                stream.write_all(response.as_bytes()).unwrap();
            });
            endpoint
        }
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let good = serve("200 OK", r#"{"responseStatus":200,"responseData":{"translatedText":"你好 &amp; 欢迎"}}"#);
        let result = tauri::async_runtime::block_on(translate_mymemory_at(&client, &good, "Hello", "en", "zh"));
        assert_eq!(result.unwrap(), "你好 & 欢迎");
        let limited = serve("429 Too Many Requests", "{}");
        let result = tauri::async_runtime::block_on(translate_mymemory_at(&client, &limited, "Hello", "en", "zh"));
        assert!(result.unwrap_err().contains("免费额度"));
        let limited_json = serve("200 OK", r#"{"responseStatus":429,"responseData":{"translatedText":"WARNING"}}"#);
        let result = tauri::async_runtime::block_on(translate_mymemory_at(&client, &limited_json, "Hello", "en", "zh"));
        assert!(result.unwrap_err().contains("免费额度"));
    }
}

#[derive(Serialize)]
struct FreeModel { id: String, name: String }

#[tauri::command]
async fn refresh_free_models(state: State<'_, AppState>) -> Result<Vec<FreeModel>, String> {
    let response = state.client.get("https://openrouter.ai/api/v1/models").send().await.map_err(|_| "无法连接 OpenRouter，请检查网络后重试")?;
    if !response.status().is_success() { return Err(format!("模型目录请求失败（HTTP {}）", response.status())); }
    let body: serde_json::Value = response.json().await.map_err(|_| "模型目录格式无法识别")?;
    let data = body.get("data").and_then(|v| v.as_array()).ok_or("模型目录格式无法识别")?;
    let mut results = Vec::new();
    for model in data {
        let Some(id) = model.get("id").and_then(|v| v.as_str()) else { continue };
        let zero = |key: &str| model.pointer(&format!("/pricing/{key}")).and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok()) == Some(0.0);
        // OpenRouter currently omits the optional request field for many :free
        // variants. Their official :free guarantee covers model inference;
        // if the field is present, require an explicit zero rather than guessing.
        let request_ok = model.pointer("/pricing/request").map_or(true, |v| v.as_str().and_then(|v| v.parse::<f64>().ok()) == Some(0.0));
        let text_input = model.pointer("/architecture/input_modalities").and_then(|v| v.as_array()).map_or(false, |a| a.iter().any(|v| v == "text"));
        let text_output = model.pointer("/architecture/output_modalities").and_then(|v| v.as_array()).map_or(false, |a| a.iter().any(|v| v == "text"));
        if id.ends_with(":free") && zero("prompt") && zero("completion") && request_ok && text_input && text_output {
            results.push(FreeModel { id: id.to_string(), name: model.get("name").and_then(|v| v.as_str()).unwrap_or(id).to_string() });
        }
    }
    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

#[tauri::command]
fn get_pending_selection(state: State<'_, AppState>) -> Result<String, String> { state.pending.lock().map(|s| s.clone()).map_err(|_| "读取选区失败".into()) }

#[tauri::command]
fn copy_text(app: tauri::AppHandle, text: String) -> Result<(), String> { app.clipboard().write_text(text).map_err(|e| format!("复制失败：{e}")) }

#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let allowed = ["https://siliconflow.cn/", "https://cloud.siliconflow.cn/", "https://docs.siliconflow.cn/", "https://help.aliyun.com/", "https://mt.console.aliyun.com/", "https://openrouter.ai/", "https://ai.baidu.com/", "https://mymemory.translated.net/"];
    if !allowed.iter().any(|base| url.starts_with(base)) { return Err("只能打开列出的官方服务页面".into()); }
    app.opener().open_url(url, None::<&str>).map_err(|e| format!("无法打开浏览器：{e}"))
}

#[tauri::command]
fn show_main(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn dismiss_overlay(window: tauri::WebviewWindow) { if window.label() == "overlay" || window.label() == "translation" { let _ = window.hide(); } }

#[tauri::command]
fn open_translation(app: tauri::AppHandle) -> Result<(), String> {
    let overlay = app.get_webview_window("overlay").ok_or("图标窗口不存在")?;
    let position = overlay.outer_position().map_err(|e| e.to_string())?;
    let _ = overlay.hide();
    let window = app.get_webview_window("translation").ok_or("翻译窗口不存在")?;
    let (x, y) = clamp_position(&window, position.x, position.y, 420, 350);
    window.set_position(Position::Physical(PhysicalPosition::new(x, y))).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    state.translation_pending.store(true, Ordering::SeqCst);
    if state.translation_ready.load(Ordering::SeqCst) && state.translation_pending.swap(false, Ordering::SeqCst) {
        window.emit("selection", ()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn translation_ready(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    if window.label() != "translation" { return Err("窗口无权注册翻译事件".into()); }
    state.translation_ready.store(true, Ordering::SeqCst);
    if state.translation_pending.swap(false, Ordering::SeqCst) { window.emit("selection", ()).map_err(|e| e.to_string())?; }
    Ok(())
}

fn clamp_position(window: &tauri::WebviewWindow, x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    if let Ok(monitors) = window.available_monitors() {
        if let Some(m) = monitors.iter().find(|m| { let p = m.position(); let s = m.size(); x >= p.x && x < p.x + s.width as i32 && y >= p.y && y < p.y + s.height as i32 }) {
            // Window sizes are logical pixels; captured mouse and work-area coordinates
            // are physical pixels. Scale for the destination display, not the display
            // on which this hidden window happened to be created.
            let area = m.work_area();
            let scale = m.scale_factor();
            let physical_width = (width as f64 * scale).ceil() as i32;
            let physical_height = (height as f64 * scale).ceil() as i32;
            let p = area.position; let s = area.size;
            return (x.clamp(p.x, (p.x + s.width as i32 - physical_width).max(p.x)), y.clamp(p.y, (p.y + s.height as i32 - physical_height).max(p.y)));
        }
    }
    (x, y)
}

fn main() {
    if let Ok(store) = windows_native_keyring_store::Store::new() { keyring_core::set_default_store(store); }
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("settings.json");
            let settings: Settings = fs::read(&config_path).ok().and_then(|v| serde_json::from_slice(&v).ok()).unwrap_or_else(|| {
                let mut settings = Settings::default();
                // Earlier versions could save a key without writing settings.json.
                if matches!(key_entry().and_then(|entry| entry.get_password().map_err(|e| e.to_string())), Ok(key) if !key.is_empty()) {
                    settings.provider = legacy_provider();
                }
                settings
            });
            capture::set_enabled(settings.enabled);
            capture::set_excluded_apps(settings.excluded_apps.clone());
            app.manage(AppState { settings: Mutex::new(settings), pending: Mutex::new(String::new()), credential_lock: Mutex::new(()), capture_error: Mutex::new(None), translation_ready: AtomicBool::new(false), translation_pending: AtomicBool::new(false), client: reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?, config_path });

            WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html?window=overlay".into()))
                .title("一划").inner_size(48.0, 48.0).decorations(false).transparent(true).always_on_top(true).skip_taskbar(true).resizable(false).visible(false).focused(false).build()?;
            WebviewWindowBuilder::new(app, "translation", WebviewUrl::App("index.html?window=translation".into()))
                .title("一划翻译").inner_size(420.0, 350.0).decorations(false).always_on_top(true).skip_taskbar(true).resizable(false).visible(false).build()?;
            let show = MenuItem::with_id(app, "show", "打开一划", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new().tooltip("一划 · 划词翻译").menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() { "show" => { let _ = show_main(app.clone()); }, "quit" => app.exit(0), _ => {} })
                .on_tray_icon_event(|tray, event| { if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event { let _ = show_main(tray.app_handle().clone()); } });
            if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
            tray.build(app)?;
            let handle = app.handle().clone();
            if let Err(error) = capture::start(move |event| {
                match event {
                    capture::CaptureEvent::Selection(selection) => {
                        let state = handle.state::<AppState>();
                        if let Ok(mut pending) = state.pending.lock() { *pending = selection.text; }
                        if let Some(window) = handle.get_webview_window("overlay") {
                            let (x, y) = clamp_position(&window, selection.x + 12, selection.y + 12, 48, 48);
                            let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
                            let _ = window.show();
                        }
                    }
                    capture::CaptureEvent::Dismiss => { if let Some(window) = handle.get_webview_window("overlay") { let _ = window.hide(); } }
                }
            }) { *app.state::<AppState>().capture_error.lock().unwrap() = Some(error); }
            Ok(())
        })
        .on_window_event(|window, event| { if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); } })
        .invoke_handler(tauri::generate_handler![get_settings, save_settings, set_api_key, clear_api_key, translate_text, refresh_free_models, get_pending_selection, copy_text, open_url, show_main, dismiss_overlay, open_translation, translation_ready])
        .run(tauri::generate_context!())
        .expect("启动一划失败");
}
