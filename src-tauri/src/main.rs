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
    #[serde(default)]
    pub ac: Option<String>,
    #[serde(default)]
    pub light: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ha_url: String,
    pub token: String,
    pub pc_entity_id: String,
    #[serde(default)]
    pub entity_id: Option<DeviceIds>,
}

impl AppConfig {
    fn ac_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.ac.as_deref())
    }

    fn light_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.light.as_deref())
    }
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
    #[serde(rename = "acAvailable")]
    pub ac_available: bool,
    #[serde(rename = "lightAvailable")]
    pub light_available: bool,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct HaRequest {
    pub url: String,
    pub body: serde_json::Value,
}

const WM_QUERYENDSESSION_MESSAGE: u32 = 0x0011;

#[derive(Debug, Deserialize)]
struct HaEntityState {
    state: String,
    #[serde(default)]
    attributes: serde_json::Value,
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
            Self::ToggleAc { on } => {
                let entity_id = config
                    .ac_entity_id()
                    .ok_or_else(|| anyhow!("AC entity is not configured"))?;
                Ok(HaRequest {
                    url: format!(
                        "{base}/api/services/climate/{}",
                        if *on { "turn_on" } else { "turn_off" }
                    ),
                    body: json!({"entity_id": entity_id}),
                })
            }
            Self::SetTemp { temp } => {
                let entity_id = config
                    .ac_entity_id()
                    .ok_or_else(|| anyhow!("AC entity is not configured"))?;
                Ok(HaRequest {
                    url: format!("{base}/api/services/climate/set_temperature"),
                    body: json!({
                        "entity_id": entity_id,
                        "temperature": temp,
                    }),
                })
            }
            Self::ToggleLight { on } => {
                let entity_id = config
                    .light_entity_id()
                    .ok_or_else(|| anyhow!("light entity is not configured"))?;
                Ok(HaRequest {
                    url: format!(
                        "{base}/api/services/light/{}",
                        if *on { "turn_on" } else { "turn_off" }
                    ),
                    body: json!({"entity_id": entity_id}),
                })
            }
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
async fn fetch_ha_entity_state(config: &AppConfig, entity_id: &str) -> Result<HaEntityState> {
    let base = config.ha_url.trim_end_matches('/');
    Ok(ha_client()
        .get(format!("{base}/api/states/{entity_id}"))
        .bearer_auth(&config.token)
        .send()
        .await?
        .error_for_status()?
        .json::<HaEntityState>()
        .await?)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse_temperature(attributes: &serde_json::Value) -> Option<i32> {
    for key in ["temperature", "target_temperature", "current_temperature"] {
        if let Some(value) = attributes.get(key) {
            if let Some(temp) = value.as_i64() {
                return Some(temp as i32);
            }
            if let Some(temp) = value.as_f64() {
                return Some(temp.round() as i32);
            }
        }
    }

    None
}

#[cfg_attr(not(windows), allow(dead_code))]
fn snapshot_from_ha_state(
    pc_state: &HaEntityState,
    ac_state: Option<&HaEntityState>,
    light_state: Option<&HaEntityState>,
) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = pc_state.state.eq_ignore_ascii_case("on");
    snapshot.ac_available = ac_state.is_some();
    snapshot.light_available = light_state.is_some();

    if let Some(ac_state) = ac_state {
        snapshot.ac.is_on = !ac_state.state.eq_ignore_ascii_case("off");

        if let Some(temp) = parse_temperature(&ac_state.attributes) {
            snapshot.ac.temp = temp;
        }
    } else {
        snapshot.ac.is_on = false;
    }

    if let Some(light_state) = light_state {
        snapshot.light_on = light_state.state.eq_ignore_ascii_case("on");
    } else {
        snapshot.light_on = false;
    }

    snapshot
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn snapshot_from_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: &serde_json::Value,
    light_state: &serde_json::Value,
) -> Result<DeviceSnapshot> {
    let pc_state: HaEntityState = serde_json::from_value(pc_state.clone())?;
    let ac_state: HaEntityState = serde_json::from_value(ac_state.clone())?;
    let light_state: HaEntityState = serde_json::from_value(light_state.clone())?;

    Ok(snapshot_from_ha_state(&pc_state, Some(&ac_state), Some(&light_state)))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn snapshot_from_optional_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    light_state: Option<&serde_json::Value>,
) -> Result<DeviceSnapshot> {
    let pc_state: HaEntityState = serde_json::from_value(pc_state.clone())?;
    let ac_state = ac_state.map(|value| serde_json::from_value(value.clone())).transpose()?;
    let light_state = light_state.map(|value| serde_json::from_value(value.clone())).transpose()?;

    Ok(snapshot_from_ha_state(
        &pc_state,
        ac_state.as_ref(),
        light_state.as_ref(),
    ))
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
    let mut first_err: Option<anyhow::Error> = None;

    if let Err(err) = send_ha_notification(config, true).await {
        first_err = Some(err);
    }

    if config.ac_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleAc { on: true }).await {
            first_err.get_or_insert(err);
        }
    }

    if config.light_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleLight { on: true }).await {
            first_err.get_or_insert(err);
        }
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
async fn send_shutdown_signal(config: &AppConfig) -> Result<()> {
    send_ha_notification(config, false).await
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
async fn fetch_current_snapshot(config: &AppConfig) -> Result<DeviceSnapshot> {
    let pc_state = fetch_ha_entity_state(config, &config.pc_entity_id).await?;
    let ac_state = match config.ac_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let light_state = match config.light_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };

    Ok(snapshot_from_ha_state(
        &pc_state,
        ac_state.as_ref(),
        light_state.as_ref(),
    ))
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

fn tolerate_autostart_error(message: &str) -> bool {
    message.contains("os error 5")
        || message.contains("Access is denied")
        || message.contains("拒绝访问")
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
        ac_available: true,
        light_available: true,
        connected: true,
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn offline_snapshot(config: &AppConfig) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = false;
    snapshot.ac_available = config.ac_entity_id().is_some();
    snapshot.light_available = config.light_entity_id().is_some();

    if !snapshot.ac_available {
        snapshot.ac.is_on = false;
    }

    if !snapshot.light_available {
        snapshot.light_on = false;
    }

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
            if config.ac_entity_id().is_none() {
                return Ok(snapshot);
            }
            let next = !snapshot.ac.is_on;
            send_ha_request(
                config,
                HaAction::ToggleAc { on: next }.into_request(config)?,
            )
            .await?;
            snapshot.ac.is_on = next;
        }
        "ac_set_temp" => {
            if config.ac_entity_id().is_none() {
                return Ok(snapshot);
            }
            let temp = args.value.ok_or_else(|| anyhow!("missing temperature"))?;
            send_ha_request(config, HaAction::SetTemp { temp }.into_request(config)?).await?;
            snapshot.ac.temp = temp;
        }
        "light_toggle" => {
            if config.light_entity_id().is_none() {
                return Ok(snapshot);
            }
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
            snapshot.ac_available = config.ac_entity_id().is_some();
            snapshot.light_available = config.light_entity_id().is_some();

            if snapshot.ac_available {
                snapshot.ac.is_on = true;
            } else {
                snapshot.ac.is_on = false;
            }

            if snapshot.light_available {
                snapshot.light_on = true;
            } else {
                snapshot.light_on = false;
            }
        }
        "shutdown_signal" => {
            send_shutdown_signal(config).await?;
        }
        _ => return Err(anyhow!("unsupported action: {}", args.action)),
    }

    Ok(snapshot)
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
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, SetWindowLongPtrW, GWLP_WNDPROC, WM_NCDESTROY, WNDPROC,
    };
    use winreg::{enums::*, RegKey};

    pub struct SharedState(pub Mutex<DeviceSnapshot>);

    static ORIGINAL_WNDPROCS: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();

    fn wndproc_store() -> &'static Mutex<HashMap<isize, isize>> {
        ORIGINAL_WNDPROCS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn hwnd_store_key(hwnd: HWND) -> isize {
        hwnd_store_key_from_raw(hwnd as usize)
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
                let _ = tauri::async_runtime::block_on(send_shutdown_signal(&config));
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
            let prev = SetWindowLongPtrW(
                hwnd,
                GWLP_WNDPROC,
                main_window_proc as *const () as isize,
            );
            let prev = set_window_long_ptr_result(prev, GetLastError())?;
            let mut store = wndproc_store().lock().map_err(|e| e.to_string())?;
            store.insert(hwnd_key, prev);
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
                if let Err(err) = write_autostart_registry_entry(true) {
                    // Some locked-down Windows installs deny HKCU Run writes; startup should still continue.
                    if tolerate_autostart_error(&err) {
                        eprintln!("autostart enable skipped: {err}");
                        return Ok(());
                    }
                    return Err(anyhow!(err));
                }

                if let Err(err) = verify_autostart_registry_entry() {
                    if tolerate_autostart_error(&err) {
                        eprintln!("autostart verification skipped: {err}");
                        return Ok(());
                    }
                    return Err(anyhow!(err));
                }

                Ok(())
            },
            || send_startup_online(&config),
        )
        .await
        {
                Ok(()) => match fetch_current_snapshot(&config).await {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        eprintln!("failed to fetch current snapshot: {err}");
                        offline_snapshot(&config)
                }
            },
            Err(err) => {
                eprintln!("startup bootstrap failed: {err}");
                offline_snapshot(&config)
            }
        };
        {
            let mut state = state.0.lock().map_err(|e| e.to_string())?;
            *state = snapshot.clone();
        }
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
        let snapshot = {
            let state = state.0.lock().map_err(|e| e.to_string())?;
            state.clone()
        };
        let next = apply_action(&config, snapshot, ActionArgs { action, value })
            .await
            .map_err(|e| e.to_string())?;
        {
            let mut state = state.0.lock().map_err(|e| e.to_string())?;
            *state = next.clone();
        }
        emit_state_refresh(&app, &next);
        Ok(next)
    }

    fn tray_menu() -> SystemTrayMenu {
        SystemTrayMenu::new()
            .add_item(CustomMenuItem::new("show".to_string(), "打开"))
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
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
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
        build_notification_request, handle_windows_message_kind, hwnd_store_key_from_raw,
        initial_snapshot, notification_timeout, offline_snapshot, query_end_session_result_value,
        resolve_config_path, run_best_effort_three, run_serialized_tray_action,
        set_window_long_ptr_result, tolerate_autostart_error,
        snapshot_from_home_assistant, snapshot_from_optional_home_assistant, AppConfig,
        DeviceIds, DeviceSnapshot, HaAction,
    };
    use anyhow::anyhow;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex as StdMutex};
    use std::time::Duration;

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
        let mut reader = decoder
            .read_info()
            .unwrap_or_else(|error| panic!("directory entry {index} PNG payload should decode: {error}"));
        let mut decoded = vec![0; reader.output_buffer_size()];
        let frame = reader
            .next_frame(&mut decoded)
            .unwrap_or_else(|error| panic!("directory entry {index} PNG payload should read a frame: {error}"));

        assert_eq!(frame.width, width, "directory entry {index} PNG width should match the ICO directory");
        assert_eq!(frame.height, height, "directory entry {index} PNG height should match the ICO directory");
        assert!(
            frame.buffer_size() > 0,
            "directory entry {index} PNG frame should decode pixel data"
        );
    }

    fn assert_dib_payload(index: usize, payload: &[u8], width: u32, height: u32, bit_count: u16) {
        assert!(payload.len() >= 40, "directory entry {index} DIB payload should include a BITMAPINFOHEADER");

        let header_size = read_le_u32(payload, 0) as usize;
        assert!(header_size >= 40, "directory entry {index} DIB header should be at least a BITMAPINFOHEADER");
        assert!(payload.len() >= header_size, "directory entry {index} DIB payload should include the full header");

        let dib_width = read_le_i32(payload, 4);
        let dib_height = read_le_i32(payload, 8);
        let planes = read_le_u16(payload, 12);
        let dib_bit_count = read_le_u16(payload, 14);
        let compression = read_le_u32(payload, 16);
        let declared_bitmap_bytes = read_le_u32(payload, 20) as usize;

        assert_eq!(planes, 1, "directory entry {index} DIB payload should declare one plane");
        assert_eq!(dib_width.unsigned_abs(), width, "directory entry {index} DIB width should match the ICO directory");
        assert_ne!(dib_height, 0, "directory entry {index} DIB height should not be zero");
        assert_eq!(dib_height % 2, 0, "directory entry {index} DIB height should include XOR and AND masks");
        assert_eq!(dib_height.unsigned_abs() / 2, height, "directory entry {index} DIB height should match the ICO directory once the mask row is excluded");
        assert_eq!(dib_bit_count, bit_count, "directory entry {index} DIB bit depth should match the ICO directory");
        assert!(dib_bit_count >= 8, "directory entry {index} DIB should not use a low-color placeholder format");
        assert!(matches!(compression, 0 | 3), "directory entry {index} DIB compression should be BI_RGB or BI_BITFIELDS");
        assert!(payload.len() > header_size, "directory entry {index} DIB payload should contain bitmap data after the header");
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
        assert_eq!(config.pc_entity_id, "input_boolean.pc_05_online");
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

        assert_eq!(config.pc_entity_id, "input_boolean.pc_05_online");
        assert_eq!(config.entity_id, None);
        assert_eq!(config.ac_entity_id(), None);
        assert_eq!(config.light_entity_id(), None);
    }

    #[test]
    fn builds_air_conditioning_turn_on_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
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
    fn builds_air_conditioning_turn_off_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = HaAction::ToggleAc { on: false }
            .into_request(&config)
            .expect("request");

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
    fn builds_shutdown_signal_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = build_notification_request(&config, false).expect("notification request");

        assert_eq!(
            request.url,
            "https://ha.example.local/api/services/input_boolean/turn_off"
        );
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "input_boolean.pc_05_online"})
        );
    }

    #[test]
    fn builds_online_notification_request_for_pc_entity() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
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
    fn builds_light_turn_off_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let request = HaAction::ToggleLight { on: false }
            .into_request(&config)
            .expect("request");

        assert_eq!(request.url, "https://ha.example.local/api/services/light/turn_off");
        assert_eq!(
            request.body,
            serde_json::json!({"entity_id": "light.office_light"})
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

    #[test]
    fn windows_icon_file_declares_valid_multisize_images() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
        let bytes = std::fs::read(&icon_path).expect("icon should exist");

        assert!(bytes.len() >= 6, "icon should include the ico header");
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0, "reserved field should be zero");
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1, "icon should declare ico resource type");

        let image_count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert!(image_count >= 4, "icon should provide multiple image sizes for Windows packaging");

        let directory_len = 6 + image_count * 16;
        assert!(bytes.len() >= directory_len, "icon directory should fit inside the file");

        let mut sizes = Vec::with_capacity(image_count);
        for index in 0..image_count {
            let entry_offset = 6 + index * 16;
            let width = if bytes[entry_offset] == 0 { 256 } else { bytes[entry_offset] as u32 };
            let height = if bytes[entry_offset + 1] == 0 { 256 } else { bytes[entry_offset + 1] as u32 };
            let color_count = bytes[entry_offset + 2];
            let reserved = bytes[entry_offset + 3];
            let bit_count = read_le_u16(&bytes, entry_offset + 6);
            let image_size = read_le_u32(&bytes, entry_offset + 8) as usize;
            let image_offset = read_le_u32(&bytes, entry_offset + 12) as usize;

            assert_eq!(reserved, 0, "directory entry {index} reserved field should be zero");
            assert_eq!(color_count, 0, "directory entry {index} should use true-color image data");
            assert!(bit_count >= 8, "directory entry {index} should not use a low-color placeholder format");
            assert!(image_size > 0, "directory entry {index} image payload should not be empty");
            assert!(image_offset >= directory_len, "directory entry {index} payload should start after the directory");
            assert!(
                image_offset.checked_add(image_size).is_some_and(|end| end <= bytes.len()),
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
                _ => panic!("directory entry {index} should begin with PNG data or a BITMAPINFOHEADER"),
            }

            sizes.push((width, height));
        }

        assert!(sizes.contains(&(16, 16)), "icon should include a 16x16 image");
        assert!(sizes.contains(&(32, 32)), "icon should include a 32x32 image");
        assert!(sizes.contains(&(48, 48)), "icon should include a 48x48 image");
        assert!(sizes.contains(&(256, 256)), "icon should include a 256x256 image");
    }

    #[test]
    fn windows_tray_icon_png_uses_rgba_pixels() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let decoder = png::Decoder::new(
            std::fs::File::open(&icon_path).expect("tray icon PNG should exist"),
        );
        let reader = decoder
            .read_info()
            .expect("tray icon PNG should decode");

        assert_eq!(
            reader.info().color_type,
            png::ColorType::Rgba,
            "tray icon PNG should use RGBA pixels for Tauri Windows metadata generation"
        );
    }

    #[test]
    fn windows_tray_icon_png_matches_expected_tray_asset_dimensions() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let decoder = png::Decoder::new(
            std::fs::File::open(&icon_path).expect("tray icon PNG should exist"),
        );
        let reader = decoder
            .read_info()
            .expect("tray icon PNG should decode");

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
        let err = set_window_long_ptr_result(0, 5).expect_err(
            "a zero SetWindowLongPtrW result with a non-zero last error should fail",
        );

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

    #[test]
    fn offline_snapshot_marks_disconnected() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: "input_boolean.pc_05_online".into(),
            entity_id: None,
        };

        let snapshot = offline_snapshot(&config);

        assert!(!snapshot.connected);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn builds_snapshot_from_home_assistant_entity_states() {
        let pc_state = serde_json::json!({
            "state": "on",
            "attributes": {}
        });
        let ac_state = serde_json::json!({
            "state": "cool",
            "attributes": {
                "temperature": 24
            }
        });
        let light_state = serde_json::json!({
            "state": "off",
            "attributes": {}
        });

        let snapshot = snapshot_from_home_assistant(&pc_state, &ac_state, &light_state)
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(snapshot.ac_available);
        assert!(snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn builds_snapshot_when_ac_and_light_are_missing() {
        let pc_state = serde_json::json!({
            "state": "on",
            "attributes": {}
        });

        let snapshot = snapshot_from_optional_home_assistant(&pc_state, None, None)
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light_on);
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
