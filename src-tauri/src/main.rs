#![cfg_attr(windows, windows_subsystem = "windows")]

mod action;
mod commands;
mod ha_client;
mod models;
mod snapshot;
mod temperature;

use anyhow::{anyhow, Result};
use std::future::Future;
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use models::{AppConfig, DeviceIds, DeviceSnapshot};
use snapshot::initial_snapshot;
#[cfg(windows)]
use snapshot::offline_snapshot;

#[cfg(windows)]
pub(crate) struct SharedState(pub std::sync::Mutex<DeviceSnapshot>);

const WM_QUERYENDSESSION_MESSAGE: u32 = 0x0011;

pub fn resolve_user_app_dir_from_base_dir(base_local_dir: &Path) -> PathBuf {
    base_local_dir.join("cyber-link")
}

pub fn ensure_user_app_dir_from_base_dir(base_local_dir: &Path) -> Result<PathBuf> {
    let app_dir = base_local_dir.join("cyber-link");
    fs::create_dir_all(&app_dir)?;
    Ok(app_dir)
}

pub fn resolve_user_config_path_from_base_dir(base_local_dir: &Path) -> PathBuf {
    base_local_dir.join("cyber-link").join("config.json")
}

pub fn resolve_user_log_path_from_base_dir(base_local_dir: &Path) -> PathBuf {
    base_local_dir.join("cyber-link").join("app.log")
}

pub fn resolve_user_config_path() -> Result<PathBuf> {
    let base =
        directories::BaseDirs::new().ok_or_else(|| anyhow!("failed to resolve user directory"))?;
    Ok(resolve_user_config_path_from_base_dir(
        base.data_local_dir(),
    ))
}

pub fn resolve_user_log_path() -> Result<PathBuf> {
    let base =
        directories::BaseDirs::new().ok_or_else(|| anyhow!("failed to resolve user directory"))?;
    Ok(resolve_user_log_path_from_base_dir(base.data_local_dir()))
}

pub fn ensure_user_app_dir() -> Result<PathBuf> {
    let base =
        directories::BaseDirs::new().ok_or_else(|| anyhow!("failed to resolve user directory"))?;
    ensure_user_app_dir_from_base_dir(base.data_local_dir())
}

pub fn load_config() -> Result<AppConfig> {
    let path = resolve_user_config_path()?;
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

#[allow(dead_code)]
pub(crate) fn append_log_line(line: &str) -> Result<()> {
    let path = resolve_user_log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn log_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs(), now.subsec_millis())
}

#[allow(dead_code)]
pub(crate) fn log_line(level: &str, line: impl AsRef<str>) {
    let line = format!("{} [{}] {}", log_timestamp(), level, line.as_ref());
    let _ = append_log_line(&line);
    eprintln!("{line}");
}

#[cfg_attr(not(test), allow(dead_code))]
static TRAY_ACTION_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

#[cfg_attr(not(test), allow(dead_code))]
async fn run_serialized_tray_action<F, Fut>(action: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    let lock = TRAY_ACTION_LOCK.get_or_init(|| tokio::sync::Mutex::new(()));
    let _guard = lock.lock().await;
    action().await;
}

#[cfg_attr(not(windows), allow(dead_code))]
#[cfg_attr(not(test), allow(dead_code))]
async fn run_best_effort_three<F1, F2, F3, Fut1, Fut2, Fut3>(
    first: F1,
    second: F2,
    third: F3,
) -> Result<()>
where
    F1: FnOnce() -> Fut1,
    F2: FnOnce() -> Fut2,
    F3: FnOnce() -> Fut3,
    Fut1: Future<Output = Result<()>>,
    Fut2: Future<Output = Result<()>>,
    Fut3: Future<Output = Result<()>>,
{
    let mut first_err: Option<anyhow::Error> = None;

    if let Err(err) = first().await {
        first_err = Some(err);
    }
    if let Err(err) = second().await {
        first_err.get_or_insert(err);
    }
    if let Err(err) = third().await {
        first_err.get_or_insert(err);
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupMode {
    Manual,
    Autostart,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupWindowAction {
    Show,
    Hide,
}

pub fn startup_mode_from_args<I, S>(args: I) -> StartupMode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if args
        .into_iter()
        .skip(1)
        .any(|arg| arg.as_ref() == "--autostart")
    {
        StartupMode::Autostart
    } else {
        StartupMode::Manual
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn startup_window_action(mode: StartupMode) -> StartupWindowAction {
    match mode {
        StartupMode::Manual => StartupWindowAction::Show,
        StartupMode::Autostart => StartupWindowAction::Hide,
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) async fn retry_startup_task<T, F, Fut>(max_attempts: usize, mut task: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_err = None;
    for _ in 0..max_attempts {
        match task().await {
            Ok(value) => return Ok(value),
            Err(err) => last_err = Some(err),
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("startup task failed")))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) async fn refresh_snapshot_with_retry(config: &AppConfig) -> Result<DeviceSnapshot> {
    retry_startup_task(3, || {
        let config = config.clone();
        async move { crate::action::fetch_current_snapshot(&config).await }
    })
    .await
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn bootstrap_startup_mode<E, S, F>(
    mode: StartupMode,
    enable_autostart: E,
    startup_online: S,
) -> Result<()>
where
    E: FnOnce() -> Result<()>,
    S: FnOnce() -> F,
    F: Future<Output = Result<()>>,
{
    match mode {
        StartupMode::Manual => Ok(()),
        StartupMode::Autostart => bootstrap_default_startup(enable_autostart, startup_online).await,
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn bootstrap_default_startup<E, S, F>(enable_autostart: E, startup_online: S) -> Result<()>
where
    E: FnOnce() -> Result<()>,
    S: FnOnce() -> F,
    F: Future<Output = Result<()>>,
{
    enable_autostart()?;
    startup_online().await?;
    Ok(())
}

pub(crate) fn tolerate_autostart_error(message: &str) -> bool {
    message.contains("os error 5")
        || message.contains("Access is denied")
        || message.contains("拒绝访问")
}

pub fn autostart_registry_value(exe_path: &Path) -> String {
    format!("\"{}\" --autostart", exe_path.display())
}

#[cfg_attr(not(windows), allow(dead_code))]
#[cfg_attr(not(test), allow(dead_code))]
async fn apply_tray_toggle<T>(
    result: Result<T>,
    mut snapshot: DeviceSnapshot,
    mutate: impl FnOnce(&mut DeviceSnapshot),
) -> Result<DeviceSnapshot> {
    result?;
    mutate(&mut snapshot);
    Ok(snapshot)
}

fn hwnd_store_key_from_raw(hwnd: usize) -> isize {
    hwnd as isize
}

fn query_end_session_result_value() -> isize {
    1
}

fn handle_windows_message_kind(msg: u32) -> bool {
    msg == WM_QUERYENDSESSION_MESSAGE
}

fn set_window_long_ptr_result(prev: isize, last_error: u32) -> Result<isize, String> {
    if prev == 0 && last_error != 0 {
        return Err(format!(
            "SetWindowLongPtrW failed: {}",
            std::io::Error::from_raw_os_error(last_error as i32)
        ));
    }

    Ok(prev)
}

fn try_restore_existing_window<F, R, L, P>(
    attempts: usize,
    mut find_window: F,
    mut restore_window: R,
    mut log_missing_window: L,
    mut pause_between_attempts: P,
) -> bool
where
    F: FnMut() -> Option<usize>,
    R: FnMut(usize),
    L: FnMut(),
    P: FnMut(),
{
    for _ in 0..attempts {
        if let Some(hwnd) = find_window() {
            restore_window(hwnd);
            return true;
        }

        pause_between_attempts();
    }

    log_missing_window();
    true
}

fn main_window_title() -> &'static str {
    static TITLE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    TITLE
        .get_or_init(|| {
            serde_json::from_str::<serde_json::Value>(include_str!("../tauri.conf.json"))
                .ok()
                .and_then(|config| {
                    config
                        .get("tauri")?
                        .get("windows")?
                        .as_array()?
                        .first()?
                        .get("title")?
                        .as_str()
                        .map(|title| title.to_string())
                })
                .unwrap_or_else(|| "CyberControl HA Client".to_string())
        })
        .as_str()
}

#[cfg(windows)]
mod windows_app {
    use super::*;
    use crate::commands::{
        append_log_message, handle_ha_action, initialize_app, is_autostart_mode, refresh_ha_state,
        set_autostart_enabled,
    };
    use serde::Deserialize;
    use std::{
        collections::HashMap,
        mem,
        sync::{Mutex, OnceLock},
        time::Duration,
    };
    use tauri::{
        AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
        WindowEvent,
    };
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, FindWindowW, SetForegroundWindow, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, GWLP_WNDPROC, SWP_NOACTIVATE, SWP_NOZORDER, SW_RESTORE,
        WM_NCDESTROY, WNDPROC,
    };
    use winreg::enums::*;

    #[derive(Deserialize)]
    struct WindowSize {
        width: f64,
        height: f64,
    }

    static ORIGINAL_WNDPROCS: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();
    static INSTANCE_MUTEX: OnceLock<usize> = OnceLock::new();
    const TRAY_OPEN_ID: &str = "tray-open";
    const TRAY_EXIT_ID: &str = "tray-exit";

    fn wndproc_store() -> &'static Mutex<HashMap<isize, isize>> {
        ORIGINAL_WNDPROCS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn hwnd_store_key(hwnd: HWND) -> isize {
        hwnd_store_key_from_raw(hwnd as usize)
    }

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn main_window_size() -> Result<WindowSize, String> {
        serde_json::from_str(include_str!("../../src/shared/windowSize.json")).map_err(|err| {
            let message = format!("failed to load main window size: {err}");
            log_line("ERROR", &message);
            message
        })
    }

    fn apply_main_window_size(window: &tauri::Window) {
        if let Ok(size) = main_window_size() {
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: size.width,
                height: size.height,
            }));
        }
    }

    unsafe fn apply_main_window_size_to_hwnd(hwnd: HWND) {
        if let Ok(size) = main_window_size() {
            let _ = SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                size.width as i32,
                size.height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }

    fn show_main_window(app: &AppHandle) {
        if let Some(window) = app.get_window("main") {
            apply_main_window_size(&window);
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    fn build_tray() -> SystemTray {
        let open = CustomMenuItem::new(TRAY_OPEN_ID.to_string(), "打开");
        let exit = CustomMenuItem::new(TRAY_EXIT_ID.to_string(), "退出");

        SystemTray::new().with_menu(SystemTrayMenu::new().add_item(open).add_item(exit))
    }

    fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                TRAY_OPEN_ID => show_main_window(app),
                TRAY_EXIT_ID => app.exit(0),
                _ => {}
            },
            SystemTrayEvent::DoubleClick { .. } => show_main_window(app),
            _ => {}
        }
    }

    fn try_restore_existing_main_window() -> bool {
        const INSTANCE_MUTEX_NAME: &str = "Local\\CyberControl_HA_Client_SingleInstance";

        unsafe {
            SetLastError(0);
            let mutex_name = to_wide(INSTANCE_MUTEX_NAME);
            let mutex = CreateMutexW(std::ptr::null_mut(), 1, mutex_name.as_ptr());

            if mutex.is_null() {
                return false;
            }

            let _ = INSTANCE_MUTEX.set(mutex as usize);

            if GetLastError() == ERROR_ALREADY_EXISTS {
                let window_title = to_wide(main_window_title());
                return try_restore_existing_window(
                    3,
                    || {
                        let hwnd = FindWindowW(std::ptr::null(), window_title.as_ptr());
                        (!hwnd.is_null()).then_some(hwnd as usize)
                    },
                    |hwnd| {
                        let hwnd = hwnd as HWND;
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        unsafe { apply_main_window_size_to_hwnd(hwnd) };
                        let _ = SetForegroundWindow(hwnd);
                    },
                    || {
                        log_line(
                            "WARN",
                            "single-instance mutex exists but main window was not found; skipping duplicate startup",
                        );
                    },
                    || std::thread::sleep(std::time::Duration::from_millis(50)),
                );
            }
        }

        false
    }

    fn query_end_session_result() -> LRESULT {
        query_end_session_result_value() as LRESULT
    }

    unsafe extern "system" fn main_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if handle_windows_message_kind(msg) {
            if let Ok(config) = load_config() {
                let _ =
                    tauri::async_runtime::block_on(crate::action::send_shutdown_signal(&config));
            }
            return query_end_session_result();
        }

        let prev = wndproc_store()
            .lock()
            .ok()
            .and_then(|store| store.get(&hwnd_store_key(hwnd)).copied());

        if msg == WM_NCDESTROY {
            if let Ok(mut store) = wndproc_store().lock() {
                store.remove(&hwnd_store_key(hwnd));
            }
        }

        if let Some(prev_proc) = prev {
            let prev_proc: WNDPROC = mem::transmute(prev_proc);
            return CallWindowProcW(prev_proc, hwnd, msg, wparam, lparam);
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn install_shutdown_hook(window: &tauri::Window) -> Result<(), String> {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let hwnd = hwnd.0 as HWND;
        let hwnd_key = hwnd_store_key(hwnd);
        unsafe {
            SetLastError(0);
            let prev =
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, main_window_proc as *const () as isize);
            let prev = set_window_long_ptr_result(prev, GetLastError())?;
            let mut store = wndproc_store().lock().map_err(|e| e.to_string())?;
            store.insert(hwnd_key, prev);
        }
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn handle_windows_message(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        if handle_windows_message_kind(msg) {
            let _ = (wparam, lparam);
            Some(query_end_session_result())
        } else {
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{handle_windows_message, hwnd_store_key, query_end_session_result};
        use windows_sys::Win32::Foundation::{HWND, LPARAM};
        use windows_sys::Win32::UI::WindowsAndMessaging::WM_QUERYENDSESSION;

        #[test]
        fn hwnd_store_key_uses_raw_pointer_hwnd_shape() {
            let hwnd = 0x1234usize as HWND;

            assert_eq!(hwnd_store_key(hwnd), 0x1234isize);
        }

        #[test]
        fn query_end_session_result_returns_true_lresult_alias() {
            assert_eq!(query_end_session_result(), 1);
        }

        #[test]
        fn handle_windows_message_returns_query_end_session_success() {
            let result = handle_windows_message(WM_QUERYENDSESSION, 0, 0 as LPARAM);

            assert_eq!(result, Some(1));
        }
    }

    pub fn run() {
        if try_restore_existing_main_window() {
            return;
        }

        let startup_mode = startup_mode_from_args(std::env::args());
        tauri::Builder::default()
            .system_tray(build_tray())
            .on_system_tray_event(handle_tray_event)
            .on_window_event(|event| {
                if let WindowEvent::CloseRequested { api, .. } = event.event() {
                    api.prevent_close();
                    let _ = event.window().hide();
                }
            })
            .manage(SharedState(Mutex::new(initial_snapshot())))
            .setup(move |app| {
                if let Some(window) = app.get_window("main") {
                    if let Err(err) = install_shutdown_hook(&window) {
                        log_line("ERROR", format!("failed to install shutdown hook: {err}"));
                    }
                }
                match startup_mode {
                    StartupMode::Autostart => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.hide();
                        }
                    }
                    StartupMode::Manual => {}
                }
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                is_autostart_mode,
                initialize_app,
                refresh_ha_state,
                handle_ha_action,
                set_autostart_enabled,
                append_log_message
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

#[cfg(windows)]
fn main() {
    windows_app::run();
}

#[cfg(not(windows))]
fn main() {}

#[cfg(test)]
mod tests {
    use super::{
        apply_tray_toggle, autostart_registry_value, bootstrap_default_startup,
        bootstrap_startup_mode, ensure_user_app_dir_from_base_dir, handle_windows_message_kind,
        hwnd_store_key_from_raw, initial_snapshot, main_window_title,
        query_end_session_result_value, refresh_snapshot_with_retry,
        resolve_user_app_dir_from_base_dir, resolve_user_config_path_from_base_dir,
        resolve_user_log_path_from_base_dir, retry_startup_task, run_best_effort_three,
        run_serialized_tray_action, set_window_long_ptr_result, startup_mode_from_args,
        startup_window_action, tolerate_autostart_error, try_restore_existing_window, AppConfig,
        DeviceIds, DeviceSnapshot, StartupMode, StartupWindowAction,
    };
    use anyhow::anyhow;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex,
    };
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn read_le_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn read_le_i32(bytes: &[u8], offset: usize) -> i32 {
        i32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn assert_png_payload(index: usize, payload: &[u8], width: u32, height: u32) {
        let decoder = png::Decoder::new(Cursor::new(payload));
        let mut reader = decoder.read_info().unwrap_or_else(|error| {
            panic!("directory entry {index} PNG payload should decode: {error}")
        });
        let mut decoded = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut decoded).unwrap_or_else(|error| {
            panic!("directory entry {index} PNG payload should read a frame: {error}")
        });

        assert_eq!(
            frame.width, width,
            "directory entry {index} PNG width should match the ICO directory"
        );
        assert_eq!(
            frame.height, height,
            "directory entry {index} PNG height should match the ICO directory"
        );
        assert!(
            frame.buffer_size() > 0,
            "directory entry {index} PNG frame should decode pixel data"
        );
    }

    fn assert_dib_payload(index: usize, payload: &[u8], width: u32, height: u32, bit_count: u16) {
        assert!(
            payload.len() >= 40,
            "directory entry {index} DIB payload should include a BITMAPINFOHEADER"
        );

        let header_size = read_le_u32(payload, 0) as usize;
        assert!(
            header_size >= 40,
            "directory entry {index} DIB header should be at least a BITMAPINFOHEADER"
        );
        assert!(
            payload.len() >= header_size,
            "directory entry {index} DIB payload should include the full header"
        );

        let dib_width = read_le_i32(payload, 4);
        let dib_height = read_le_i32(payload, 8);
        let planes = read_le_u16(payload, 12);
        let dib_bit_count = read_le_u16(payload, 14);
        let compression = read_le_u32(payload, 16);
        let declared_bitmap_bytes = read_le_u32(payload, 20) as usize;

        assert_eq!(
            planes, 1,
            "directory entry {index} DIB payload should declare one plane"
        );
        assert_eq!(
            dib_width.unsigned_abs(),
            width,
            "directory entry {index} DIB width should match the ICO directory"
        );
        assert_ne!(
            dib_height, 0,
            "directory entry {index} DIB height should not be zero"
        );
        assert_eq!(
            dib_height % 2,
            0,
            "directory entry {index} DIB height should include XOR and AND masks"
        );
        assert_eq!(dib_height.unsigned_abs() / 2, height, "directory entry {index} DIB height should match the ICO directory once the mask row is excluded");
        assert_eq!(
            dib_bit_count, bit_count,
            "directory entry {index} DIB bit depth should match the ICO directory"
        );
        assert!(
            dib_bit_count >= 8,
            "directory entry {index} DIB should not use a low-color placeholder format"
        );
        assert!(
            matches!(compression, 0 | 3),
            "directory entry {index} DIB compression should be BI_RGB or BI_BITFIELDS"
        );
        assert!(
            payload.len() > header_size,
            "directory entry {index} DIB payload should contain bitmap data after the header"
        );
        assert!(
            declared_bitmap_bytes == 0 || declared_bitmap_bytes <= payload.len() - header_size,
            "directory entry {index} DIB declared bitmap length should fit inside the payload"
        );
    }

    #[test]
    fn parses_nested_entity_ids_from_config_json() {
        let json = r#"{
            "ha_url": "https://ha.example.local",
            "token": "secret",
            "pc_entity_id": "input_boolean.pc_05_online",
            "entity_id": {
                "ac": "climate.office_ac",
                "light": "light.office_light"
            }
        }"#;

        let config: AppConfig = serde_json::from_str(json).expect("config should parse");

        assert_eq!(config.ha_url, "https://ha.example.local");
        assert_eq!(config.token, "secret");
        assert_eq!(
            config.pc_entity_id.as_deref(),
            Some("input_boolean.pc_05_online")
        );
        assert_eq!(
            config.entity_id,
            Some(DeviceIds {
                ac: Some("climate.office_ac".to_string()),
                light: Some("light.office_light".to_string()),
            })
        );
    }

    #[test]
    fn parses_config_when_ac_and_light_are_missing() {
        let json = r#"{
            "ha_url": "https://ha.example.local",
            "token": "secret",
            "pc_entity_id": "input_boolean.pc_05_online"
        }"#;

        let config: AppConfig = serde_json::from_str(json).expect("config should parse");

        assert_eq!(
            config.pc_entity_id.as_deref(),
            Some("input_boolean.pc_05_online")
        );
        assert_eq!(config.entity_id, None);
        assert_eq!(config.ac_entity_id(), None);
        assert_eq!(config.light_entity_id(), None);
    }

    #[test]
    fn parses_config_without_pc_entity_id() {
        let json = r#"{
            "ha_url": "https://ha.example.local",
            "token": "secret",
            "entity_id": {
                "ac": "climate.office_ac",
                "light": "light.office_light"
            }
        }"#;

        let config: AppConfig = serde_json::from_str(json).expect("config should parse");

        assert_eq!(config.ha_url, "https://ha.example.local");
        assert_eq!(config.token, "secret");
        assert!(config.pc_entity_id.is_none());
    }

    #[test]
    fn builds_climate_turn_on_request_in_ha_client() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = crate::ha_client::climate_turn_on_request(&config).expect("request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/climate/turn_on"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "climate.office_ac"})
        );
    }

    #[test]
    fn builds_climate_turn_off_request_in_ha_client() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = crate::ha_client::climate_turn_off_request(&config).expect("request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/climate/turn_off"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "climate.office_ac"})
        );
    }

    #[test]
    fn builds_light_turn_on_request_in_ha_client() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = crate::ha_client::light_turn_on_request(&config).expect("request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/light/turn_on"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "light.office_light"})
        );
    }

    #[test]
    fn builds_light_turn_off_request_in_ha_client() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = crate::ha_client::light_turn_off_request(&config).expect("request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/light/turn_off"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "light.office_light"})
        );
    }

    #[test]
    fn normalizes_climate_temperature_with_step_and_clamp() {
        let state = crate::models::HaEntityState {
            state: "cool".into(),
            attributes: serde_json::json!({
                "temperature": 21,
                "min_temp": 16,
                "max_temp": 28,
                "step": 2,
                "temperature_unit": "°C"
            }),
        };

        assert_eq!(
            crate::ha_client::normalize_climate_temperature(&state, 15),
            16.0
        );
        assert_eq!(
            crate::ha_client::normalize_climate_temperature(&state, 23),
            24.0
        );
        assert_eq!(
            crate::ha_client::normalize_climate_temperature(&state, 31),
            28.0
        );
    }

    #[tokio::test]
    async fn builds_climate_set_temperature_request_from_current_state() {
        std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
        std::env::set_var("no_proxy", "127.0.0.1,localhost");
        std::env::set_var("HTTP_PROXY", "");
        std::env::set_var("HTTPS_PROXY", "");
        std::env::set_var("http_proxy", "");
        std::env::set_var("https_proxy", "");

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.expect("read request");
            let request = String::from_utf8_lossy(&buf[..n]);
            assert!(request.contains("GET /api/states/climate.office_ac HTTP/1.1"));

            let body = r#"{"state":"cool","attributes":{"temperature":21,"min_temp":16,"max_temp":28,"step":2,"temperature_unit":"°C"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let normalized = crate::ha_client::normalized_climate_temperature(&config, 23)
            .await
            .expect("normalized temperature");
        let request = crate::ha_client::climate_set_temperature_request(&config, normalized)
            .expect("request");

        server.await.expect("server task");

        assert_eq!(
            request.url,
            format!("http://{addr}/api/services/climate/set_temperature")
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "climate.office_ac", "temperature": 24})
        );
    }

    #[test]
    fn resolves_config_path_from_user_local_app_data() {
        let resolved =
            resolve_user_config_path_from_base_dir(Path::new(r"C:\Users\Alice\AppData\Local"));

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Alice\AppData\Local")
                .join("cyber-link")
                .join("config.json")
        );
    }

    #[test]
    fn resolves_log_path_from_user_local_app_data() {
        let resolved =
            resolve_user_log_path_from_base_dir(Path::new(r"C:\Users\Alice\AppData\Local"));

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Alice\AppData\Local")
                .join("cyber-link")
                .join("app.log")
        );
    }

    #[test]
    fn resolves_app_dir_from_user_local_app_data() {
        let resolved =
            resolve_user_app_dir_from_base_dir(Path::new(r"C:\Users\Alice\AppData\Local"));

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Alice\AppData\Local").join("cyber-link")
        );
    }

    #[test]
    fn ensure_app_dir_creates_the_directory() {
        let temp = tempdir().expect("temp dir");
        let base = temp.path().join("Local");
        std::fs::create_dir_all(&base).expect("base dir");

        let resolved = ensure_user_app_dir_from_base_dir(&base).expect("should create app dir");

        assert_eq!(resolved, base.join("cyber-link"));
        assert!(resolved.exists());
        assert!(resolved.is_dir());
    }

    #[tokio::test]
    async fn startup_retry_helper_retries_three_times() {
        let attempts = Arc::new(StdMutex::new(0));
        let attempts_for_call = Arc::clone(&attempts);

        let result = retry_startup_task(3, move || {
            let attempts_for_call = Arc::clone(&attempts_for_call);
            async move {
                let mut guard = attempts_for_call.lock().unwrap();
                *guard += 1;
                Err::<(), anyhow::Error>(anyhow!("temporary failure"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*attempts.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn refresh_snapshot_with_retry_surfaces_failure() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: None,
        };

        let result = refresh_snapshot_with_retry(&config).await;

        assert!(result.is_err());
    }

    #[test]
    fn parses_autostart_mode_from_args() {
        assert!(matches!(
            startup_mode_from_args(["CyberControl_HA_Client.exe", "--autostart"]),
            StartupMode::Autostart
        ));
        assert!(matches!(
            startup_mode_from_args(["CyberControl_HA_Client.exe"]),
            StartupMode::Manual
        ));
    }

    #[test]
    fn manual_startup_shows_the_main_window() {
        assert_eq!(
            startup_window_action(StartupMode::Manual),
            StartupWindowAction::Show
        );
    }

    #[test]
    fn autostart_hides_the_main_window() {
        assert_eq!(
            startup_window_action(StartupMode::Autostart),
            StartupWindowAction::Hide
        );
    }

    #[test]
    fn startup_path_returns_single_visible_main_window() {
        assert_eq!(
            startup_window_action(StartupMode::Manual),
            StartupWindowAction::Show
        );
        assert_eq!(
            startup_window_action(StartupMode::Autostart),
            StartupWindowAction::Hide
        );

        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri config should parse");
        let windows = config
            .get("tauri")
            .and_then(|tauri| tauri.get("windows"))
            .and_then(|windows| windows.as_array())
            .expect("tauri config should declare windows");

        assert_eq!(
            windows.len(),
            1,
            "app should keep a single main window definition"
        );
        let main_window = windows
            .first()
            .expect("tauri config should declare one window");
        assert_eq!(
            main_window.get("label").and_then(|label| label.as_str()),
            Some("main")
        );
        assert_eq!(
            main_window
                .get("visible")
                .and_then(|visible| visible.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn main_window_title_matches_tauri_config() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri config should parse");
        let expected = config
            .get("tauri")
            .and_then(|tauri| tauri.get("windows"))
            .and_then(|windows| windows.as_array())
            .and_then(|windows| windows.first())
            .and_then(|window| window.get("title"))
            .and_then(|title| title.as_str())
            .expect("tauri config should declare a window title");

        assert_eq!(main_window_title(), expected);
    }

    #[test]
    fn single_instance_restore_retries_before_succeeding() {
        let attempts = Arc::new(StdMutex::new(0usize));
        let restored = Arc::new(StdMutex::new(Vec::new()));

        let result = try_restore_existing_window(
            3,
            {
                let attempts = Arc::clone(&attempts);
                move || {
                    let mut attempts = attempts.lock().unwrap();
                    *attempts += 1;
                    if *attempts < 2 {
                        None
                    } else {
                        Some(0x1234)
                    }
                }
            },
            {
                let restored = Arc::clone(&restored);
                move |hwnd| restored.lock().unwrap().push(hwnd)
            },
            || panic!("should not log missing window when a retry succeeds"),
            || {},
        );

        assert!(result);
        assert_eq!(*attempts.lock().unwrap(), 2);
        assert_eq!(restored.lock().unwrap().as_slice(), &[0x1234]);
    }

    #[test]
    fn single_instance_restore_fails_closed_when_window_never_appears() {
        let restored = Arc::new(StdMutex::new(Vec::new()));
        let missing = Arc::new(AtomicBool::new(false));

        let result = try_restore_existing_window(
            2,
            || None,
            {
                let restored = Arc::clone(&restored);
                move |hwnd| restored.lock().unwrap().push(hwnd)
            },
            {
                let missing = Arc::clone(&missing);
                move || missing.store(true, Ordering::SeqCst)
            },
            || {},
        );

        assert!(result);
        assert!(restored.lock().unwrap().is_empty());
        assert!(missing.load(Ordering::SeqCst));
    }

    #[test]
    fn builds_registry_value_from_executable_path() {
        let value = autostart_registry_value(Path::new(r"C:\Program Files\Cyber\cyber-link.exe"));

        assert_eq!(
            value,
            r#""C:\Program Files\Cyber\cyber-link.exe" --autostart"#
        );
    }

    #[test]
    fn windows_icon_file_declares_valid_multisize_images() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
        let bytes = std::fs::read(&icon_path).expect("icon should exist");

        assert!(bytes.len() >= 6, "icon should include the ico header");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            0,
            "reserved field should be zero"
        );
        assert_eq!(
            u16::from_le_bytes([bytes[2], bytes[3]]),
            1,
            "icon should declare ico resource type"
        );

        let image_count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert!(
            image_count >= 4,
            "icon should provide multiple image sizes for Windows packaging"
        );

        let directory_len = 6 + image_count * 16;
        assert!(
            bytes.len() >= directory_len,
            "icon directory should fit inside the file"
        );

        let mut sizes = Vec::with_capacity(image_count);
        for index in 0..image_count {
            let entry_offset = 6 + index * 16;
            let width = if bytes[entry_offset] == 0 {
                256
            } else {
                bytes[entry_offset] as u32
            };
            let height = if bytes[entry_offset + 1] == 0 {
                256
            } else {
                bytes[entry_offset + 1] as u32
            };
            let color_count = bytes[entry_offset + 2];
            let reserved = bytes[entry_offset + 3];
            let bit_count = read_le_u16(&bytes, entry_offset + 6);
            let image_size = read_le_u32(&bytes, entry_offset + 8) as usize;
            let image_offset = read_le_u32(&bytes, entry_offset + 12) as usize;

            assert_eq!(
                reserved, 0,
                "directory entry {index} reserved field should be zero"
            );
            assert_eq!(
                color_count, 0,
                "directory entry {index} should use true-color image data"
            );
            assert!(
                bit_count >= 8,
                "directory entry {index} should not use a low-color placeholder format"
            );
            assert!(
                image_size > 0,
                "directory entry {index} image payload should not be empty"
            );
            assert!(
                image_offset >= directory_len,
                "directory entry {index} payload should start after the directory"
            );
            assert!(
                image_offset
                    .checked_add(image_size)
                    .is_some_and(|end| end <= bytes.len()),
                "directory entry {index} payload should fit inside the file"
            );

            let image_end = image_offset + image_size;
            let payload = &bytes[image_offset..image_end];
            match payload {
                [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => {
                    assert_png_payload(index, payload, width, height);
                }
                [40, 0, 0, 0, ..] => {
                    assert_dib_payload(index, payload, width, height, bit_count);
                }
                _ => panic!(
                    "directory entry {index} should begin with PNG data or a BITMAPINFOHEADER"
                ),
            }

            sizes.push((width, height));
        }

        assert!(
            sizes.contains(&(16, 16)),
            "icon should include a 16x16 image"
        );
        assert!(
            sizes.contains(&(32, 32)),
            "icon should include a 32x32 image"
        );
        assert!(
            sizes.contains(&(48, 48)),
            "icon should include a 48x48 image"
        );
        assert!(
            sizes.contains(&(256, 256)),
            "icon should include a 256x256 image"
        );
    }

    #[test]
    fn windows_tray_icon_png_uses_rgba_pixels() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let decoder =
            png::Decoder::new(std::fs::File::open(&icon_path).expect("tray icon PNG should exist"));
        let reader = decoder.read_info().expect("tray icon PNG should decode");

        assert_eq!(
            reader.info().color_type,
            png::ColorType::Rgba,
            "tray icon PNG should use RGBA pixels for Tauri Windows metadata generation"
        );
    }

    #[test]
    fn windows_tray_icon_png_matches_expected_tray_asset_dimensions() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let decoder =
            png::Decoder::new(std::fs::File::open(&icon_path).expect("tray icon PNG should exist"));
        let reader = decoder.read_info().expect("tray icon PNG should decode");

        assert_eq!(
            reader.info().width,
            32,
            "tray icon PNG should stay 32px wide for Windows tray metadata generation"
        );
        assert_eq!(
            reader.info().height,
            32,
            "tray icon PNG should stay 32px tall for Windows tray metadata generation"
        );
    }

    #[test]
    fn set_window_long_ptr_result_treats_zero_with_error_as_failure() {
        let err = set_window_long_ptr_result(0, 5)
            .expect_err("a zero SetWindowLongPtrW result with a non-zero last error should fail");

        assert!(
            err.contains("SetWindowLongPtrW"),
            "error should identify the failing Windows API call"
        );
    }

    #[test]
    fn set_window_long_ptr_result_accepts_zero_without_error() {
        assert_eq!(
            set_window_long_ptr_result(0, 0).expect(
                "a zero SetWindowLongPtrW result with a cleared last error should be accepted"
            ),
            0
        );
    }

    #[test]
    fn shared_shutdown_message_helper_identifies_query_end_session() {
        assert!(handle_windows_message_kind(0x0011));
        assert!(!handle_windows_message_kind(0x0082));
    }

    #[test]
    fn shared_shutdown_message_helper_returns_success_value() {
        assert_eq!(query_end_session_result_value(), 1);
    }

    #[test]
    fn shared_shutdown_message_helper_preserves_raw_hwnd_value() {
        assert_eq!(hwnd_store_key_from_raw(0x1234usize), 0x1234isize);
    }

    #[test]
    fn invoke_handler_command_names_remain_stable() {
        assert_eq!(
            crate::commands::INVOKE_HANDLER_COMMAND_NAMES,
            [
                "is_autostart_mode",
                "initialize_app",
                "refresh_ha_state",
                "handle_ha_action",
                "set_autostart_enabled",
                "append_log_message",
            ]
        );
    }

    #[tokio::test]
    async fn boots_autostart_before_startup_action() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_for_enable = Arc::clone(&calls);
        let calls_for_startup = Arc::clone(&calls);

        bootstrap_default_startup(
            move || {
                calls_for_enable.lock().unwrap().push("autostart");
                Ok(())
            },
            move || {
                calls_for_startup.lock().unwrap().push("startup");
                async { Ok::<(), anyhow::Error>(()) }
            },
        )
        .await
        .expect("startup bootstrap should succeed");

        assert_eq!(calls.lock().unwrap().as_slice(), ["autostart", "startup"]);
    }

    #[tokio::test]
    async fn startup_fails_when_autostart_fails() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_for_startup = Arc::clone(&calls);

        let result = bootstrap_default_startup(
            || Err(anyhow!("registry write failed")),
            move || {
                calls_for_startup.lock().unwrap().push("startup");
                async { Ok::<(), anyhow::Error>(()) }
            },
        )
        .await;

        assert!(result.is_err());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn manual_startup_skips_boot_automation() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_for_enable = Arc::clone(&calls);
        let calls_for_startup = Arc::clone(&calls);

        bootstrap_startup_mode(
            StartupMode::Manual,
            move || {
                calls_for_enable.lock().unwrap().push("autostart");
                Ok(())
            },
            move || {
                calls_for_startup.lock().unwrap().push("startup");
                async { Ok::<(), anyhow::Error>(()) }
            },
        )
        .await
        .expect("manual startup should succeed");

        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn autostart_runs_setup_before_startup_action() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_for_enable = Arc::clone(&calls);
        let calls_for_startup = Arc::clone(&calls);

        bootstrap_startup_mode(
            StartupMode::Autostart,
            move || {
                calls_for_enable.lock().unwrap().push("autostart");
                Ok(())
            },
            move || {
                calls_for_startup.lock().unwrap().push("startup");
                async { Ok::<(), anyhow::Error>(()) }
            },
        )
        .await
        .expect("autostart startup should succeed");

        assert_eq!(calls.lock().unwrap().as_slice(), ["autostart", "startup"]);
    }

    #[test]
    fn tolerate_autostart_error_allows_windows_access_denied() {
        assert!(tolerate_autostart_error("拒绝访问 (os error 5)"));
        assert!(tolerate_autostart_error("Access is denied. (os error 5)"));
        assert!(!tolerate_autostart_error("registry value mismatch"));
    }

    #[tokio::test]
    async fn startup_fails_when_online_action_fails() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_for_startup = Arc::clone(&calls);

        let result = bootstrap_default_startup(
            || Ok(()),
            move || {
                calls_for_startup.lock().unwrap().push("startup");
                async { Err::<(), anyhow::Error>(anyhow!("ha down")) }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(calls.lock().unwrap().as_slice(), ["startup"]);
    }

    #[tokio::test]
    async fn best_effort_sequence_keeps_running_after_failure() {
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let first_calls = Arc::clone(&calls);
        let second_calls = Arc::clone(&calls);
        let third_calls = Arc::clone(&calls);

        let result = run_best_effort_three(
            move || {
                first_calls.lock().unwrap().push("first");
                async { Err::<(), anyhow::Error>(anyhow!("first failed")) }
            },
            move || {
                second_calls.lock().unwrap().push("second");
                async { Ok::<(), anyhow::Error>(()) }
            },
            move || {
                third_calls.lock().unwrap().push("third");
                async { Ok::<(), anyhow::Error>(()) }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["first", "second", "third"]
        );
    }

    #[tokio::test]
    async fn tray_toggle_does_not_mutate_on_failure() {
        let snapshot = initial_snapshot();
        let result: anyhow::Result<DeviceSnapshot> = apply_tray_toggle(
            Err::<(), anyhow::Error>(anyhow!("ha failed")),
            snapshot.clone(),
            |s| {
                s.light_on = !s.light_on;
            },
        )
        .await;

        assert!(result.is_err());
        assert!(snapshot.light_on);
    }

    #[tokio::test]
    async fn tray_toggle_mutates_after_success() {
        let snapshot = initial_snapshot();
        let next = apply_tray_toggle(Ok::<(), anyhow::Error>(()), snapshot, |s| {
            s.ac.is_on = false;
        })
        .await
        .expect("toggle should succeed");

        assert!(!next.ac.is_on);
    }

    #[tokio::test]
    async fn serialized_tray_actions_wait_for_each_other() {
        let entered_second = Arc::new(AtomicBool::new(false));
        let first_released = Arc::new(tokio::sync::Notify::new());
        let first_started = Arc::new(tokio::sync::Notify::new());

        let entered_second_clone = Arc::clone(&entered_second);
        let first_released_clone = Arc::clone(&first_released);
        let first_started_clone = Arc::clone(&first_started);

        let first = tokio::spawn(async move {
            run_serialized_tray_action(|| async {
                first_started_clone.notify_one();
                first_released_clone.notified().await;
            })
            .await;
        });

        first_started.notified().await;

        let second = tokio::spawn(async move {
            run_serialized_tray_action(|| async {
                entered_second_clone.store(true, Ordering::SeqCst);
            })
            .await;
        });

        tokio::task::yield_now().await;
        assert!(!entered_second.load(Ordering::SeqCst));

        first_released.notify_one();
        let _ = first.await;
        let _ = second.await;

        assert!(entered_second.load(Ordering::SeqCst));
    }
}
