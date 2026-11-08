use serde::Serialize;
use std::{io, path::Path};
use winreg::{enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE}, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const ENTRY: &str = "com.wordglance.app";
pub const ARGUMENT: &str = "--autostart";

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status { Disabled, Enabled, Stale, Blocked }

fn command_for(path: &Path) -> Result<String, String> {
    let path = path.to_str().ok_or("程序路径包含无法识别的字符")?;
    let command = format!("\"{path}\" {ARGUMENT}");
    if command.encode_utf16().count() > 260 { return Err("程序路径过长，Windows 无法登记开机启动；请将程序移至较短的路径".into()); }
    Ok(command)
}

fn classify(value: Option<&str>, expected: &str, blocked: bool) -> Status {
    match value {
        None => Status::Disabled,
        Some(value) if !value.eq_ignore_ascii_case(expected) => Status::Stale,
        Some(_) if blocked => Status::Blocked,
        Some(_) => Status::Enabled,
    }
}

fn read_value(path: &str, name: &str) -> io::Result<Option<String>> {
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => match key.get_value(name) { Ok(value) => Ok(Some(value)), Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None), Err(e) => Err(e) },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn windows_blocked() -> io::Result<bool> {
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(APPROVED_KEY, KEY_READ) {
        Ok(key) => match key.get_raw_value(ENTRY) {
            Ok(value) => Ok(matches!(value.bytes.first(), Some(3 | 7))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn status() -> Result<Status, String> {
    let expected = command_for(&std::env::current_exe().map_err(|e| format!("无法获取程序路径：{e}"))?)?;
    let value = read_value(RUN_KEY, ENTRY).map_err(|e| format!("无法读取 Windows 启动项：{e}"))?;
    let blocked = if value.is_some() { windows_blocked().map_err(|e| format!("无法读取 Windows 启动许可：{e}"))? } else { false };
    Ok(classify(value.as_deref(), &expected, blocked))
}

pub fn set_enabled(enabled: bool) -> Result<Status, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let command = command_for(&std::env::current_exe().map_err(|e| format!("无法获取程序路径：{e}"))?)?;
        let (key, _) = hkcu.create_subkey(RUN_KEY).map_err(|e| format!("无法打开 Windows 启动项：{e}"))?;
        key.set_value(ENTRY, &command).map_err(|e| format!("无法开启开机启动：{e}"))?;
    } else {
        match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
            Ok(key) => match key.delete_value(ENTRY) { Ok(()) => (), Err(e) if e.kind() == io::ErrorKind::NotFound => (), Err(e) => return Err(format!("无法关闭开机启动：{e}")) },
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(format!("无法打开 Windows 启动项：{e}")),
        }
    }
    status()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_paths_with_spaces_and_unicode() {
        assert_eq!(command_for(Path::new(r"C:\Program Files\一划\wordglance.exe")).unwrap(), "\"C:\\Program Files\\一划\\wordglance.exe\" --autostart");
    }
    #[test]
    fn rejects_commands_over_windows_limit() { assert!(command_for(Path::new(&"a".repeat(300))).is_err()); }
    #[test]
    fn classifies_current_stale_and_blocked_entries() {
        assert_eq!(classify(None, "current", false), Status::Disabled);
        assert_eq!(classify(Some("CURRENT"), "current", false), Status::Enabled);
        assert_eq!(classify(Some("old"), "current", false), Status::Stale);
        assert_eq!(classify(Some("current"), "current", true), Status::Blocked);
    }
}
