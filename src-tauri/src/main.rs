use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIds {
    pub ac: String,
    pub light: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ha_url: String,
    pub token: String,
    pub pc_entity_id: String,
    pub entity_id: DeviceIds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ACState {
    pub is_on: bool,
    pub temp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub room: String,
    #[serde(rename = "pcId")]
    pub pc_id: String,
    pub ac: ACState,
    #[serde(rename = "lightOn")]
    pub light_on: bool,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct HaRequest {
    pub url: String,
    pub body: serde_json::Value,
}

#[cfg_attr(not(windows), allow(dead_code))]
static HA_CLIENT: OnceLock<Client> = OnceLock::new();

#[cfg_attr(not(windows), allow(dead_code))]
fn ha_client() -> &'static Client {
    HA_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .pool_max_idle_per_host(2)
            .build()
            .expect("failed to build Home Assistant client")
    })
}

#[derive(Debug, Clone)]
pub enum HaAction {
    ToggleAc { on: bool },
    SetTemp { temp: i32 },
    ToggleLight { on: bool },
    StartupOnline,
    ShutdownSignal,
}

impl HaAction {
    pub fn into_request(&self, config: &AppConfig) -> Result<HaRequest> {
        let base = config.ha_url.trim_end_matches('/');
        match self {
            Self::ToggleAc { on } => Ok(HaRequest {
                url: format!(
                    "{base}/api/services/climate/{}",
                    if *on { "turn_on" } else { "turn_off" }
                ),
                body: json!({"entity_id": config.entity_id.ac}),
            }),
            Self::SetTemp { temp } => Ok(HaRequest {
                url: format!("{base}/api/services/climate/set_temperature"),
                body: json!({
                    "entity_id": config.entity_id.ac,
                    "temperature": temp,
                }),
            }),
            Self::ToggleLight { on } => Ok(HaRequest {
                url: format!(
                    "{base}/api/services/light/{}",
                    if *on { "turn_on" } else { "turn_off" }
                ),
                body: json!({"entity_id": config.entity_id.light}),
            }),
            Self::StartupOnline | Self::ShutdownSignal => {
                Err(anyhow!("multi-entity action requires dedicated handler"))
            }
        }
    }
}

pub fn build_notification_request(config: &AppConfig, state: bool) -> Result<HaRequest> {
    let base = config.ha_url.trim_end_matches('/');
    Ok(HaRequest {
        url: format!(
            "{base}/api/services/input_boolean/{}",
            if state { "turn_on" } else { "turn_off" }
        ),
        body: json!({"entity_id": config.pc_entity_id}),
    })
}

pub fn notification_timeout(state: bool) -> Duration {
    if state {
        Duration::from_secs(2)
    } else {
        Duration::from_millis(900)
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(not(windows), allow(dead_code))]
struct ActionArgs {
    action: String,
    value: Option<i32>,
}

pub fn resolve_config_path(current_dir: &Path, executable_dir: &Path) -> Result<PathBuf> {
    let cwd = current_dir.join("config.json");
    if cwd.exists() {
        return Ok(cwd);
    }

    let exe = executable_dir.join("config.json");
    if exe.exists() {
        return Ok(exe);
    }

    Err(anyhow!(
        "config.json not found in current or executable directory"
    ))
}

fn config_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let exe_dir = std::env::current_exe()?
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(cwd.clone());
    resolve_config_path(&cwd, &exe_dir)
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_ha_request(config: &AppConfig, request: HaRequest) -> Result<()> {
    send_ha_request_with_timeout(config, request, Duration::from_secs(2)).await
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_ha_request_with_timeout(
    config: &AppConfig,
    request: HaRequest,
    timeout: Duration,
) -> Result<()> {
    ha_client()
        .post(request.url)
        .bearer_auth(&config.token)
        .json(&request.body)
        .timeout(timeout)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_ha_action(config: &AppConfig, action: HaAction) -> Result<()> {
    send_ha_request(config, action.into_request(config)?).await
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_ha_notification(config: &AppConfig, state: bool) -> Result<()> {
    let request = build_notification_request(config, state)?;
    send_ha_request_with_timeout(config, request, notification_timeout(state)).await
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_startup_online(config: &AppConfig) -> Result<()> {
    run_best_effort_three(
        || send_ha_notification(config, true),
        || send_ha_action(config, HaAction::ToggleAc { on: true }),
        || send_ha_action(config, HaAction::ToggleLight { on: true }),
    )
    .await
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_shutdown_signal(config: &AppConfig) -> Result<()> {
    let timeout = notification_timeout(false);
    run_best_effort_three(
        || send_ha_notification(config, false),
        || async {
            send_ha_request_with_timeout(
                config,
                HaAction::ToggleAc { on: false }.into_request(config)?,
                timeout,
            )
            .await
        },
        || async {
            send_ha_request_with_timeout(
                config,
                HaAction::ToggleLight { on: false }.into_request(config)?,
                timeout,
            )
            .await
        },
    )
    .await
}

#[cfg_attr(not(windows), allow(dead_code))]
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

pub fn autostart_registry_value(exe_path: &Path) -> String {
    format!("\"{}\"", exe_path.display())
}

#[cfg_attr(not(windows), allow(dead_code))]
fn initial_snapshot() -> DeviceSnapshot {
    DeviceSnapshot {
        room: "核心-01".into(),
        pc_id: "终端-05".into(),
        ac: ACState {
            is_on: true,
            temp: 16,
        },
        light_on: true,
        connected: true,
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn offline_snapshot() -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = false;
    snapshot
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn apply_action(
    config: &AppConfig,
    mut snapshot: DeviceSnapshot,
    args: ActionArgs,
) -> Result<DeviceSnapshot> {
    match args.action.as_str() {
        "ac_toggle" => {
            let next = !snapshot.ac.is_on;
            send_ha_request(
                config,
                HaAction::ToggleAc { on: next }.into_request(config)?,
            )
            .await?;
            snapshot.ac.is_on = next;
        }
        "ac_set_temp" => {
            let temp = args.value.ok_or_else(|| anyhow!("missing temperature"))?;
            send_ha_request(config, HaAction::SetTemp { temp }.into_request(config)?).await?;
            snapshot.ac.temp = temp;
        }
        "light_toggle" => {
            let next = !snapshot.light_on;
            send_ha_request(
                config,
                HaAction::ToggleLight { on: next }.into_request(config)?,
            )
            .await?;
            snapshot.light_on = next;
        }
        "startup_online" => {
            send_startup_online(config).await?;
            snapshot.ac.is_on = true;
            snapshot.light_on = true;
        }
        "shutdown_signal" => {
            send_shutdown_signal(config).await?;
            snapshot.ac.is_on = false;
            snapshot.light_on = false;
        }
        _ => return Err(anyhow!("unsupported action: {}", args.action)),
    }

    Ok(snapshot)
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn apply_tray_toggle<T>(
    result: Result<T>,
    mut snapshot: DeviceSnapshot,
    mutate: impl FnOnce(&mut DeviceSnapshot),
) -> Result<DeviceSnapshot> {
    result?;
    mutate(&mut snapshot);
    Ok(snapshot)
}

#[cfg(windows)]
mod windows_app {
    use super::*;
    use std::{
        collections::HashMap,
        mem,
        sync::{Mutex, OnceLock},
    };
    use tauri::{
        AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    };
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, SetWindowLongPtrW, GWLP_WNDPROC, WM_NCDESTROY,
        WM_QUERYENDSESSION, WNDPROC,
    };
    use winreg::{enums::*, RegKey};

    pub struct SharedState(pub Mutex<DeviceSnapshot>);

    static ORIGINAL_WNDPROCS: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();

    fn wndproc_store() -> &'static Mutex<HashMap<isize, isize>> {
        ORIGINAL_WNDPROCS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    unsafe extern "system" fn main_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_QUERYENDSESSION {
            if let Ok(config) = load_config() {
                let _ = tauri::async_runtime::block_on(send_shutdown_signal(&config));
            }
            return 1;
        }

        let prev = wndproc_store()
            .lock()
            .ok()
            .and_then(|store| store.get(&(hwnd.0 as isize)).copied());

        if msg == WM_NCDESTROY {
            if let Ok(mut store) = wndproc_store().lock() {
                store.remove(&(hwnd.0 as isize));
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
        let hwnd_raw = hwnd.0 as isize;
        unsafe {
            let prev = SetWindowLongPtrW(hwnd_raw, GWLP_WNDPROC, main_window_proc as isize);
            let mut store = wndproc_store().lock().map_err(|e| e.to_string())?;
            store.insert(hwnd_raw, prev);
        }
        Ok(())
    }

    fn write_autostart_registry_entry(enabled: bool) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let value = autostart_registry_value(&exe);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .map_err(|e| e.to_string())?;

        if enabled {
            key.set_value("CyberControl HA Client", &value)
                .map_err(|e| e.to_string())?;
        } else {
            let _ = key.delete_value("CyberControl HA Client");
        }

        Ok(())
    }

    fn verify_autostart_registry_entry() -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let expected = autostart_registry_value(&exe);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .map_err(|e| e.to_string())?;

        let actual: String = key.get_value("CyberControl HA Client").map_err(|e| e.to_string())?;
        if actual != expected {
            return Err("autostart registry value mismatch".into());
        }

        Ok(())
    }

    fn emit_state_refresh(app: &AppHandle, snapshot: &DeviceSnapshot) {
        let _ = app.emit_all("state-refresh", snapshot.clone());
    }

    #[tauri::command]
    pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
        write_autostart_registry_entry(enabled)
    }

    #[tauri::command]
    pub async fn initialize_app(
        app: AppHandle,
        state: State<'_, SharedState>,
    ) -> Result<DeviceSnapshot, String> {
        let config = load_config().map_err(|e| e.to_string())?;
        let snapshot = match bootstrap_default_startup(
            || {
                write_autostart_registry_entry(true).map_err(|e| anyhow!(e))?;
                verify_autostart_registry_entry().map_err(|e| anyhow!(e))
            },
            || send_startup_online(&config),
        )
        .await
        {
            Ok(()) => initial_snapshot(),
            Err(err) => {
                eprintln!("startup bootstrap failed: {err}");
                offline_snapshot()
            }
        };
        *state.0.lock().map_err(|e| e.to_string())? = snapshot.clone();
        emit_state_refresh(&app, &snapshot);
        Ok(snapshot)
    }

    #[tauri::command]
    pub async fn handle_ha_action(
        app: AppHandle,
        state: State<'_, SharedState>,
        action: String,
        value: Option<i32>,
    ) -> Result<DeviceSnapshot, String> {
        let config = load_config().map_err(|e| e.to_string())?;
        let snapshot = state.0.lock().map_err(|e| e.to_string())?.clone();
        let next = apply_action(&config, snapshot, ActionArgs { action, value })
            .await
            .map_err(|e| e.to_string())?;
        *state.0.lock().map_err(|e| e.to_string())? = next.clone();
        emit_state_refresh(&app, &next);
        Ok(next)
    }

    fn tray_menu() -> SystemTrayMenu {
        SystemTrayMenu::new()
            .add_item(CustomMenuItem::new("show".to_string(), "打开控制面板"))
            .add_item(CustomMenuItem::new("ac_on".to_string(), "快速启动空调"))
            .add_item(CustomMenuItem::new(
                "light_toggle".to_string(),
                "快速开关灯",
            ))
            .add_item(CustomMenuItem::new("quit".to_string(), "退出"))
    }

    fn hide_main_window(app: &tauri::App) {
        if let Some(window) = app.get_window("main") {
            let _ = window.hide();
        }
    }

    pub fn build_tray() -> SystemTray {
        SystemTray::new().with_menu(tray_menu())
    }

    pub fn handle_tray_event(app: &tauri::AppHandle, event: SystemTrayEvent) {
        if let SystemTrayEvent::MenuItemClick { id, .. } = event {
            match id.as_str() {
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "ac_on" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<SharedState>();
                        let config = match load_config() {
                            Ok(config) => config,
                            Err(_) => return,
                        };
                        let snapshot = match state.0.lock() {
                            Ok(guard) => guard.clone(),
                            Err(_) => return,
                        };
                        if let Ok(next) = apply_tray_toggle(
                            send_ha_action(&config, HaAction::ToggleAc { on: true }).await,
                            snapshot,
                            |snapshot| {
                                snapshot.ac.is_on = true;
                            },
                        )
                        .await
                        {
                            if let Ok(mut guard) = state.0.lock() {
                                *guard = next.clone();
                            }
                            emit_state_refresh(&app, &next);
                        }
                    });
                }
                "light_toggle" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<SharedState>();
                        let config = match load_config() {
                            Ok(config) => config,
                            Err(_) => return,
                        };
                        let snapshot = match state.0.lock() {
                            Ok(guard) => guard.clone(),
                            Err(_) => return,
                        };
                        let desired = !snapshot.light_on;
                        if let Ok(next) = apply_tray_toggle(
                            send_ha_action(&config, HaAction::ToggleLight { on: desired }).await,
                            snapshot,
                            move |snapshot| {
                                snapshot.light_on = desired;
                            },
                        )
                        .await
                        {
                            if let Ok(mut guard) = state.0.lock() {
                                *guard = next.clone();
                            }
                            emit_state_refresh(&app, &next);
                        }
                    });
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    }

    pub fn handle_windows_message(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        if msg == WM_QUERYENDSESSION {
            let _ = (wparam, lparam);
            Some(LRESULT(1))
        } else {
            None
        }
    }

    pub fn run() {
        tauri::Builder::default()
            .manage(SharedState(Mutex::new(initial_snapshot())))
            .system_tray(build_tray())
            .on_system_tray_event(handle_tray_event)
            .setup(|app| {
                if let Some(window) = app.get_window("main") {
                    if let Err(err) = install_shutdown_hook(&window) {
                        eprintln!("failed to install shutdown hook: {err}");
                    }
                }
                hide_main_window(app);
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                initialize_app,
                handle_ha_action,
                set_autostart_enabled
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
        build_notification_request, initial_snapshot, notification_timeout, offline_snapshot,
        resolve_config_path, run_best_effort_three, AppConfig, DeviceIds, DeviceSnapshot, HaAction,
    };
    use anyhow::anyhow;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

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
        assert_eq!(config.pc_entity_id, "input_boolean.pc_05_online");
        assert_eq!(
            config.entity_id,
            DeviceIds {
                ac: "climate.office_ac".to_string(),
                light: "light.office_light".to_string(),
            }
        );
    }

    #[test]
    fn builds_air_conditioning_turn_on_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: DeviceIds {
                ac: "climate.office_ac".into(),
                light: "light.office_light".into(),
            },
        };

        let request = HaAction::ToggleAc { on: true }
            .into_request(&config)
            .expect("request");

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
    fn builds_shutdown_signal_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: DeviceIds {
                ac: "climate.office_ac".into(),
                light: "light.office_light".into(),
            },
        };

        let request = HaAction::ToggleAc { on: false }
            .into_request(&config)
            .expect("request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/climate/turn_off"
        );
    }

    #[test]
    fn builds_online_notification_request_for_pc_entity() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: DeviceIds {
                ac: "climate.office_ac".into(),
                light: "light.office_light".into(),
            },
        };

        let request = build_notification_request(&config, true).expect("notification request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/input_boolean/turn_on"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "input_boolean.pc_05_online"})
        );
    }

    #[test]
    fn offline_notification_uses_short_timeout() {
        assert_eq!(notification_timeout(false), Duration::from_millis(900));
        assert_eq!(notification_timeout(true), Duration::from_secs(2));
    }

    #[test]
    fn resolves_config_from_executable_directory_when_working_dir_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cwd = tmp.path().join("working");
        let exe_dir = tmp.path().join("install");
        std::fs::create_dir_all(&cwd).expect("cwd");
        std::fs::create_dir_all(&exe_dir).expect("exe dir");
        std::fs::write(exe_dir.join("config.json"), "{}").expect("config");

        let resolved = resolve_config_path(&cwd, &exe_dir).expect("resolved path");

        assert_eq!(resolved, exe_dir.join("config.json"));
    }

    #[test]
    fn builds_registry_value_from_executable_path() {
        let value = autostart_registry_value(
            PathBuf::from(r"C:\Program Files\Cyber\cyber-link.exe").as_path(),
        );

        assert_eq!(value, r#""C:\Program Files\Cyber\cyber-link.exe""#);
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

    #[test]
    fn offline_snapshot_marks_disconnected() {
        let snapshot = offline_snapshot();

        assert!(!snapshot.connected);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.light_on, true);
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
}
