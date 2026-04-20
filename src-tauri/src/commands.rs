pub const INVOKE_HANDLER_COMMAND_NAMES: [&str; 6] = [
    "is_autostart_mode",
    "initialize_app",
    "refresh_ha_state",
    "handle_ha_action",
    "set_autostart_enabled",
    "append_log_message",
];

#[cfg(windows)]
use anyhow::anyhow;
#[cfg(windows)]
use tauri::{AppHandle, Manager, State};

#[cfg(windows)]
use crate::{
    action::{self, ActionArgs, ActionKind},
    append_log_line, ensure_user_app_dir, load_config, log_line, refresh_snapshot_with_retry,
    retry_startup_task, startup_mode_from_args, tolerate_autostart_error, SharedState, StartupMode,
};

#[cfg(windows)]
use crate::{models::DeviceSnapshot, snapshot::offline_snapshot};

#[cfg(windows)]
use winreg::{enums::*, RegKey};

#[cfg(windows)]
fn emit_state_refresh(app: &AppHandle, snapshot: &DeviceSnapshot) {
    let _ = app.emit_all("state-refresh", snapshot.clone());
}

#[cfg(windows)]
fn write_autostart_registry_entry(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;

    if enabled {
        key.set_value(
            "CyberControl HA Client",
            &crate::autostart_registry_value(&exe),
        )
        .map_err(|e| e.to_string())?;
    } else {
        let _ = key.delete_value("CyberControl HA Client");
    }

    Ok(())
}

#[cfg(windows)]
fn verify_autostart_registry_entry() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let expected = crate::autostart_registry_value(&exe);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;

    let actual: String = key
        .get_value("CyberControl HA Client")
        .map_err(|e| e.to_string())?;
    if actual != expected {
        return Err("autostart registry value mismatch".into());
    }

    Ok(())
}

#[cfg(windows)]
#[tauri::command]
pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    write_autostart_registry_entry(enabled)
}

#[cfg(windows)]
#[tauri::command]
pub fn append_log_message(message: String) -> Result<(), String> {
    append_log_line(&message).map_err(|e| e.to_string())
}

#[cfg(windows)]
#[tauri::command]
pub fn is_autostart_mode() -> bool {
    matches!(
        startup_mode_from_args(std::env::args()),
        StartupMode::Autostart
    )
}

#[cfg(windows)]
#[tauri::command]
pub async fn initialize_app(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<DeviceSnapshot, String> {
    ensure_user_app_dir().map_err(|e| e.to_string())?;
    let config = load_config().map_err(|e| e.to_string())?;
    let startup_mode = startup_mode_from_args(std::env::args());
    let snapshot = offline_snapshot(&config);
    {
        let mut state = state.0.lock().map_err(|e| e.to_string())?;
        *state = snapshot.clone();
    }
    emit_state_refresh(&app, &snapshot);

    let app_for_task = app.clone();
    let config_for_task = config.clone();
    tauri::async_runtime::spawn(async move {
        let result = retry_startup_task(3, || {
            let config = config_for_task.clone();
            async move {
                if matches!(startup_mode, StartupMode::Autostart) {
                    if let Err(err) = write_autostart_registry_entry(true) {
                        if tolerate_autostart_error(&err) {
                            log_line("WARN", format!("autostart enable skipped: {err}"));
                        } else {
                            return Err(anyhow!(err));
                        }
                    }

                    action::send_startup_online(&config).await?;

                    if let Err(err) = verify_autostart_registry_entry() {
                        if tolerate_autostart_error(&err) {
                            log_line("WARN", format!("autostart verification skipped: {err}"));
                        } else {
                            return Err(anyhow!(err));
                        }
                    }
                }

                action::fetch_current_snapshot(&config).await
            }
        })
        .await;

        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(err) => {
                log_line("ERROR", format!("startup bootstrap failed: {err}"));
                offline_snapshot(&config_for_task)
            }
        };

        let shared = app_for_task.state::<SharedState>();
        if let Ok(mut state) = shared.0.lock() {
            *state = snapshot.clone();
        }
        emit_state_refresh(&app_for_task, &snapshot);
    });

    Ok(snapshot)
}

#[cfg(windows)]
#[tauri::command]
pub async fn refresh_ha_state(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<DeviceSnapshot, String> {
    let config = load_config().map_err(|e| e.to_string())?;
    let snapshot = refresh_snapshot_with_retry(&config)
        .await
        .map_err(|err| err.to_string())?;

    {
        let mut state = state.0.lock().map_err(|e| e.to_string())?;
        *state = snapshot.clone();
    }
    emit_state_refresh(&app, &snapshot);

    Ok(snapshot)
}

#[cfg(windows)]
#[tauri::command]
pub async fn handle_ha_action(
    app: AppHandle,
    state: State<'_, SharedState>,
    action: ActionKind,
    value: Option<i32>,
) -> Result<DeviceSnapshot, String> {
    let config = load_config().map_err(|e| e.to_string())?;
    let snapshot = {
        let state = state.0.lock().map_err(|e| e.to_string())?;
        state.clone()
    };
    let outcome = action::apply_action(&config, snapshot, ActionArgs { action, value })
        .await
        .map_err(|e| e.to_string())?;
    {
        let mut state = state.0.lock().map_err(|e| e.to_string())?;
        *state = outcome.snapshot.clone();
    }
    emit_state_refresh(&app, &outcome.snapshot);

    if let Some(error) = outcome.error {
        Err(error)
    } else {
        Ok(outcome.snapshot)
    }
}
