use std::future::Future;

use anyhow::Result as AnyResult;
use crate::{retry_startup_task, StartupMode};

pub(crate) async fn bootstrap_startup_snapshot<T, Enable, EnableFut, Startup, StartupFut, Verify, VerifyFut, Fetch, FetchFut>(
    startup_mode: StartupMode,
    mut enable_autostart: Enable,
    mut send_startup_online: Startup,
    mut verify_autostart: Verify,
    mut fetch_snapshot: Fetch,
) -> AnyResult<T>
where
    Enable: FnMut() -> EnableFut,
    EnableFut: Future<Output = AnyResult<()>>,
    Startup: FnMut() -> StartupFut,
    StartupFut: Future<Output = AnyResult<()>>,
    Verify: FnMut() -> VerifyFut,
    VerifyFut: Future<Output = AnyResult<()>>,
    Fetch: FnMut() -> FetchFut,
    FetchFut: Future<Output = AnyResult<T>>,
{
    if matches!(startup_mode, StartupMode::Autostart) {
        retry_startup_task(3, move || enable_autostart()).await?;
        retry_startup_task(3, move || send_startup_online()).await?;
        retry_startup_task(3, move || verify_autostart()).await?;
    }

    retry_startup_task(3, move || fetch_snapshot()).await
}

#[cfg(test)]
mod tests {
    use super::bootstrap_startup_snapshot;
    use crate::StartupMode;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn autostart_setup_runs_once_when_snapshot_retry_retries() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let enable_calls = Arc::clone(&calls);
        let startup_calls = Arc::clone(&calls);
        let verify_calls = Arc::clone(&calls);
        let fetch_calls = Arc::clone(&calls);
        let enable_attempts = Arc::new(Mutex::new(0usize));
        let enable_attempts_for_call = Arc::clone(&enable_attempts);
        let verify_attempts = Arc::new(Mutex::new(0usize));
        let verify_attempts_for_call = Arc::clone(&verify_attempts);
        let fetch_attempts = Arc::new(Mutex::new(0usize));
        let fetch_attempts_for_call = Arc::clone(&fetch_attempts);

        let result = bootstrap_startup_snapshot(
            StartupMode::Autostart,
            move || {
                let enable_attempts = Arc::clone(&enable_attempts_for_call);
                let enable_calls = Arc::clone(&enable_calls);
                async move {
                    let mut attempts = enable_attempts.lock().unwrap();
                    *attempts += 1;
                    enable_calls.lock().unwrap().push("enable");
                    if *attempts < 3 {
                        Err::<(), anyhow::Error>(anyhow::anyhow!("enable failed"))
                    } else {
                        Ok::<(), anyhow::Error>(())
                    }
                }
            },
            move || {
                startup_calls.lock().unwrap().push("startup");
                async { Ok::<(), anyhow::Error>(()) }
            },
            move || {
                let verify_attempts = Arc::clone(&verify_attempts_for_call);
                let verify_calls = Arc::clone(&verify_calls);
                async move {
                    let mut attempts = verify_attempts.lock().unwrap();
                    *attempts += 1;
                    verify_calls.lock().unwrap().push("verify");
                    if *attempts < 3 {
                        Err::<(), anyhow::Error>(anyhow::anyhow!("verify failed"))
                    } else {
                        Ok::<(), anyhow::Error>(())
                    }
                }
            },
            move || {
                let fetch_attempts = Arc::clone(&fetch_attempts_for_call);
                let fetch_calls = Arc::clone(&fetch_calls);
                async move {
                    let mut attempts = fetch_attempts.lock().unwrap();
                    *attempts += 1;
                    fetch_calls.lock().unwrap().push("fetch");
                    if *attempts < 3 {
                        Err::<String, anyhow::Error>(anyhow::anyhow!("fetch failed"))
                    } else {
                        Ok::<String, anyhow::Error>("snapshot".into())
                    }
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "snapshot");
        assert_eq!(calls.lock().unwrap().as_slice(), ["enable", "enable", "enable", "startup", "verify", "verify", "verify", "fetch", "fetch", "fetch"]);
    }

    #[tokio::test]
    async fn autostart_startup_action_retries_without_repeating_setup() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let enable_calls = Arc::clone(&calls);
        let startup_calls = Arc::clone(&calls);
        let verify_calls = Arc::clone(&calls);
        let fetch_calls = Arc::clone(&calls);
        let startup_attempts = Arc::new(Mutex::new(0usize));
        let startup_attempts_for_call = Arc::clone(&startup_attempts);

        let result = bootstrap_startup_snapshot(
            StartupMode::Autostart,
            move || {
                enable_calls.lock().unwrap().push("enable");
                async { Ok::<(), anyhow::Error>(()) }
            },
            move || {
                let startup_attempts = Arc::clone(&startup_attempts_for_call);
                let startup_calls = Arc::clone(&startup_calls);
                async move {
                    let mut attempts = startup_attempts.lock().unwrap();
                    *attempts += 1;
                    startup_calls.lock().unwrap().push("startup");
                    if *attempts < 3 {
                        Err::<(), anyhow::Error>(anyhow::anyhow!("startup failed"))
                    } else {
                        Ok::<(), anyhow::Error>(())
                    }
                }
            },
            move || {
                verify_calls.lock().unwrap().push("verify");
                async { Ok::<(), anyhow::Error>(()) }
            },
            move || {
                fetch_calls.lock().unwrap().push("fetch");
                async { Ok::<String, anyhow::Error>("snapshot".into()) }
            },
        )
        .await;

        assert_eq!(result.expect("startup bootstrap should succeed"), "snapshot");
        assert_eq!(calls.lock().unwrap().as_slice(), ["enable", "startup", "startup", "startup", "verify", "fetch"]);
    }
}

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
    action::{self, ActionArgs, ActionKind, ActionTarget},
    append_log_line, ensure_user_app_dir, load_config, log_line, refresh_snapshot_with_retry,
    startup_mode_from_args, tolerate_autostart_error, SharedState,
};
#[cfg(windows)]
use crate::ha_events::spawn_state_listener_once;
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
            "CyberLink",
            &crate::autostart_registry_value(&exe),
        )
        .map_err(|e| e.to_string())?;
    } else {
        let _ = key.delete_value("CyberLink");
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
        .get_value("CyberLink")
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
    spawn_state_listener_once(app.clone(), config.clone());

    let app_for_task = app.clone();
    let config_for_task = config.clone();
    tauri::async_runtime::spawn(async move {
        let result = bootstrap_startup_snapshot(
            startup_mode,
            || async {
                if let Err(err) = write_autostart_registry_entry(true) {
                    if tolerate_autostart_error(&err) {
                        log_line("WARN", format!("autostart enable skipped: {err}"));
                        Ok(())
                    } else {
                        Err(anyhow!(err))
                    }
                } else {
                    Ok(())
                }
            },
            || {
                let config = config_for_task.clone();
                async move { action::send_startup_online(&config).await }
            },
            || async {
                if let Err(err) = verify_autostart_registry_entry() {
                    if tolerate_autostart_error(&err) {
                        log_line("WARN", format!("autostart verification skipped: {err}"));
                        Ok(())
                    } else {
                        Err(anyhow!(err))
                    }
                } else {
                    Ok(())
                }
            },
            || {
                let config = config_for_task.clone();
                async move { action::fetch_current_snapshot(&config).await }
            },
        )
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
    target: Option<ActionTarget>,
    value: Option<i32>,
) -> Result<DeviceSnapshot, String> {
    let config = load_config().map_err(|e| e.to_string())?;
    let snapshot = {
        let state = state.0.lock().map_err(|e| e.to_string())?;
        state.clone()
    };
    let outcome = action::apply_action(&config, snapshot, ActionArgs { action, target, value })
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
