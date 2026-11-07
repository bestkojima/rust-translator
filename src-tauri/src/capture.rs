//! Windows selection capture.
//!
//! The gesture filtering and UI Automation traversal are adapted from
//! Magpie (拾趣), Copyright (c) 2026 Magpie, under the MIT license:
//! <https://github.com/demo007x/Magpie/blob/main/src-tauri/src/capture/windows.rs>.
//! The full license is preserved below.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use uiautomation::core::UIAutomation;
use uiautomation::patterns::UITextPattern;
use uiautomation::types::{ControlType, Point};
use uiautomation::UIElement;
use windows::Win32::Foundation::{CloseHandle, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetMessageW, GetWindowThreadProcessId, SetWindowsHookExW,
    UnhookWindowsHookEx, WindowFromPoint, GA_ROOT, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP,
};

/// A captured selection and the mouse release position in physical screen pixels.
#[derive(Clone, Debug)]
pub struct CapturedSelection {
    pub text: String,
    pub x: i32,
    pub y: i32,
    /// Absolute executable path when Windows permits querying it.
    pub process: String,
}

/// An actionable selection or a click outside our own windows.
#[derive(Clone, Debug)]
pub enum CaptureEvent {
    Selection(CapturedSelection),
    Dismiss,
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static EXCLUDED_APPS: OnceLock<RwLock<Vec<String>>> = OnceLock::new();
static RAW_SENDER: Mutex<Option<SyncSender<MouseEvent>>> = Mutex::new(None);
static STARTED: AtomicBool = AtomicBool::new(false);

/// Enable or pause new selection detection without uninstalling the hook.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Replace the user-defined exclusion list; entries are executable names or paths.
/// The built-in exclusions remain active. Matching is case-insensitive.
pub fn set_excluded_apps(apps: Vec<String>) {
    let apps = apps
        .into_iter()
        .map(|app| app.trim().to_lowercase())
        .filter(|app| !app.is_empty())
        .collect();
    if let Ok(mut current) = EXCLUDED_APPS.get_or_init(Default::default).write() {
        *current = apps;
    }
}

const BUILT_IN_EXCLUSIONS: &[&str] = &[
    "1password.exe",
    "bitwarden.exe",
    "keepass.exe",
    "keepassxc.exe",
    "dashlane.exe",
    "cmd.exe",
    "conhost.exe",
    "powershell.exe",
    "pwsh.exe",
    "windowsterminal.exe",
    "wt.exe",
    "wezterm-gui.exe",
    "alacritty.exe",
    "kitty.exe",
];

fn is_excluded(path: &str) -> bool {
    let path = path.to_lowercase();
    let name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
    if BUILT_IN_EXCLUSIONS.contains(&name) {
        return true;
    }
    EXCLUDED_APPS
        .get_or_init(Default::default)
        .read()
        .map(|apps| apps.iter().any(|entry| entry == name || entry == &path))
        .unwrap_or(true)
}

#[derive(Clone, Copy)]
struct MouseEvent {
    message: u32,
    x: i32,
    y: i32,
    time: u32,
}

extern "system" fn mouse_hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && ENABLED.load(Ordering::Relaxed) {
        let message = message.0 as u32;
        if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONUP) && data.0 != 0 {
            // Windows owns the event structure for the duration of this callback.
            let info = unsafe { &*(data.0 as *const MSLLHOOKSTRUCT) };
            if let Ok(sender) = RAW_SENDER.try_lock() {
                if let Some(sender) = sender.as_ref() {
                    // Never block Windows' input thread on a slow UIA provider.
                    let _ = sender.try_send(MouseEvent {
                        message,
                        x: info.pt.x,
                        y: info.pt.y,
                        time: info.time,
                    });
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, WPARAM(message.0), data) }
}

/// Start one mouse-hook thread and one UI Automation worker.
///
/// Calls `on_event` on the worker thread for a selection or dismissal.
/// No clipboard access or input simulation is performed. The source application
/// must expose a UI Automation text selection. Keyboard-only selection does not
/// trigger this mouse-based detector.
pub fn start<F>(on_event: F) -> Result<(), String>
where
    F: Fn(CaptureEvent) + Send + Sync + 'static,
{
    if STARTED.swap(true, Ordering::SeqCst) {
        return Err("划词监听已经启动".into());
    }

    let (sender, receiver) = mpsc::sync_channel(64);
    let (uia_ready_tx, uia_ready_rx) = mpsc::sync_channel(1);
    let callback = Arc::new(on_event);
    let worker = std::thread::Builder::new()
        .name("selection-uia".into())
        .spawn(move || {
            let automation = match UIAutomation::new() {
                Ok(automation) => automation,
                Err(err) => {
                    let _ =
                        uia_ready_tx.send(Err(format!("初始化 Windows UI Automation 失败：{err}")));
                    return;
                }
            };
            let _ = uia_ready_tx.send(Ok(()));
            detect_loop(&automation, receiver, callback);
        });
    if let Err(err) = worker {
        STARTED.store(false, Ordering::SeqCst);
        return Err(format!("启动划词工作线程失败：{err}"));
    }
    match uia_ready_rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            STARTED.store(false, Ordering::SeqCst);
            return Err(err);
        }
        Err(err) => {
            STARTED.store(false, Ordering::SeqCst);
            return Err(format!("划词工作线程意外退出：{err}"));
        }
    }

    *RAW_SENDER
        .lock()
        .map_err(|_| "划词监听状态不可用".to_string())? = Some(sender);
    let (hook_ready_tx, hook_ready_rx) = mpsc::sync_channel(1);
    let hook_thread = std::thread::Builder::new()
        .name("selection-hook".into())
        .spawn(move || unsafe {
            let module = match GetModuleHandleW(None) {
                Ok(module) => module,
                Err(err) => {
                    let _ = hook_ready_tx.send(Err(format!("获取程序句柄失败：{err}")));
                    return;
                }
            };
            let hook =
                match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), Some(module.into()), 0) {
                    Ok(hook) => hook,
                    Err(err) => {
                        let _ = hook_ready_tx.send(Err(format!("安装鼠标监听失败：{err}")));
                        return;
                    }
                };
            let _ = hook_ready_tx.send(Ok(()));
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
            let _ = UnhookWindowsHookEx(hook);
        });
    if let Err(err) = hook_thread {
        if let Ok(mut sender) = RAW_SENDER.lock() {
            *sender = None;
        }
        STARTED.store(false, Ordering::SeqCst);
        return Err(format!("启动鼠标监听线程失败：{err}"));
    }
    match hook_ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        result => {
            if let Ok(mut sender) = RAW_SENDER.lock() {
                *sender = None;
            }
            STARTED.store(false, Ordering::SeqCst);
            Err(match result {
                Ok(Err(err)) => err,
                Err(err) => format!("鼠标监听线程意外退出：{err}"),
                Ok(Ok(())) => unreachable!(),
            })
        }
    }
}

fn detect_loop<F>(automation: &UIAutomation, receiver: Receiver<MouseEvent>, callback: Arc<F>)
where
    F: Fn(CaptureEvent) + Send + Sync + 'static,
{
    let own_pid = std::process::id();
    let double_click_ms = unsafe { GetDoubleClickTime() };
    let mut down: Option<(MouseEvent, bool)> = None;
    let mut previous_click: Option<MouseEvent> = None;
    while let Ok(event) = receiver.recv() {
        if !ENABLED.load(Ordering::Relaxed) {
            down = None;
            previous_click = None;
            continue;
        }
        match event.message {
            WM_LBUTTONDOWN => {
                let double_click = previous_click.is_some_and(|previous| {
                    event.time.wrapping_sub(previous.time) <= double_click_ms
                        && (event.x - previous.x).abs() + (event.y - previous.y).abs() < 12
                });
                previous_click = Some(event);
                down = Some((event, double_click));
            }
            WM_LBUTTONUP => {
                let Some((start, double_click)) = down.take() else {
                    continue;
                };
                let moved = (event.x - start.x).abs() + (event.y - start.y).abs() > 8;
                if !ENABLED.load(Ordering::Relaxed) {
                    continue;
                }
                // Clicking our own icon or panel must remain actionable.
                if window_process_at(start.x, start.y) == Some(own_pid)
                    || window_process_at(event.x, event.y) == Some(own_pid)
                {
                    continue;
                }
                if moved || double_click {
                    if let Some(selection) = query_selection(automation, own_pid, start, event) {
                        callback(CaptureEvent::Selection(selection));
                        continue;
                    }
                }
                callback(CaptureEvent::Dismiss);
            }
            _ => {}
        }
    }
}

fn query_selection(
    automation: &UIAutomation,
    own_pid: u32,
    start: MouseEvent,
    end: MouseEvent,
) -> Option<CapturedSelection> {
    let start_pid = window_process_at(start.x, start.y)?;
    let end_pid = window_process_at(end.x, end.y)?;
    if start_pid == own_pid || end_pid == own_pid || start_pid != end_pid {
        return None;
    }
    // Resolve and filter the process before reading any selected text.
    let process = process_path(end_pid)?;
    if is_excluded(&process) {
        return None;
    }

    let start_element = automation
        .element_from_point(Point::new(start.x, start.y))
        .ok()?;
    if start_element.is_password().unwrap_or(true) {
        return None;
    }
    if is_control_surface(start_element.get_control_type().ok()?) {
        return None;
    }
    let end_element = automation
        .element_from_point(Point::new(end.x, end.y))
        .ok()?;
    if end_element.is_password().unwrap_or(true) {
        return None;
    }
    let end_element_pid = end_element.get_process_id().ok()?;
    if end_element_pid == own_pid || end_element_pid == 0 {
        return None;
    }
    if let Ok(focused) = automation.get_focused_element() {
        if focused.get_process_id().ok() == Some(end_element_pid)
            && focused.is_password().unwrap_or(true)
        {
            return None;
        }
    }

    let text = walk_text_selection(automation, &end_element).or_else(|| {
        let focused = automation.get_focused_element().ok()?;
        if focused.get_process_id().ok()? != end_element_pid {
            return None;
        }
        walk_text_selection(automation, &focused)
    })?;
    Some(CapturedSelection {
        text,
        x: end.x,
        y: end.y,
        process,
    })
}

fn walk_text_selection(automation: &UIAutomation, start: &UIElement) -> Option<String> {
    let walker = automation.get_control_view_walker().ok()?;
    let mut element = start.clone();
    for _ in 0..24 {
        if element.is_password().unwrap_or(true) {
            return None;
        }
        if let Ok(pattern) = element.get_pattern::<UITextPattern>() {
            if let Ok(ranges) = pattern.get_selection() {
                if let Some(text) = ranges
                    .into_iter()
                    .filter_map(|range| range.get_text(-1).ok())
                    .find(|text| !text.trim().is_empty())
                {
                    return Some(text);
                }
            }
        }
        element = walker.get_parent(&element).ok()?;
    }
    None
}

fn window_process_at(x: i32, y: i32) -> Option<u32> {
    unsafe {
        let window = WindowFromPoint(POINT { x, y });
        if window.is_invalid() {
            return None;
        }
        let root = GetAncestor(window, GA_ROOT);
        if root.is_invalid() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(root, Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        (size > 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
    }
}

fn is_control_surface(control_type: ControlType) -> bool {
    matches!(
        control_type,
        ControlType::Button
            | ControlType::SplitButton
            | ControlType::CheckBox
            | ControlType::RadioButton
            | ControlType::ComboBox
            | ControlType::Slider
            | ControlType::Spinner
            | ControlType::ScrollBar
            | ControlType::Thumb
            | ControlType::ProgressBar
            | ControlType::Tab
            | ControlType::TabItem
            | ControlType::Menu
            | ControlType::MenuBar
            | ControlType::MenuItem
            | ControlType::Tree
            | ControlType::TreeItem
            | ControlType::ToolBar
            | ControlType::TitleBar
            | ControlType::StatusBar
            | ControlType::ToolTip
            | ControlType::Calendar
            | ControlType::Separator
            | ControlType::Header
            | ControlType::HeaderItem
            | ControlType::Image
    )
}
