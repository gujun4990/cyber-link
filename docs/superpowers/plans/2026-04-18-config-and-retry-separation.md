# Config And Retry Separation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** keep the UI visible even when Home Assistant is unavailable, load config from the user directory only, and retry HA startup work in the background without blocking app launch.

**Architecture:** Add a user-directory config resolver as the only production config source, then wrap HA startup calls in a small retry helper that runs in the background. Startup mode still decides whether the window is shown or hidden, but HA failures only update app state and never gate first paint.

**Tech Stack:** Rust, Tauri v1, React, Cargo tests, Node test runner

---

### Task 1: Add Config Resolver Tests

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing tests for user-directory config lookup**

```rust
    #[test]
    fn resolves_config_path_from_user_local_app_data() {
        let path = resolve_config_path_for_user_dir("C:\\Users\\Alice\\AppData\\Local");

        assert_eq!(
            path,
            PathBuf::from(r"C:\Users\Alice\AppData\Local\cyber-link\config.json")
        );
    }

    #[test]
    fn config_missing_returns_not_configured_state() {
        let snapshot = offline_snapshot_from_missing_config("核心-01", "终端-05");

        assert!(!snapshot.connected);
    }
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail**

Run: `cargo test resolves_config_path_from_user_local_app_data config_missing_returns_not_configured_state`

Expected: FAIL because the resolver helpers do not exist yet.

### Task 2: Implement User-Directory Config Loading

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement the user-directory config resolver and missing-config fallback**

```rust
pub fn resolve_user_config_path() -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("failed to resolve user directory"))?;
    Ok(base
        .data_local_dir()
        .join("cyber-link")
        .join("config.json"))
}

fn load_config_from_user_dir() -> Result<AppConfig> {
    let path = resolve_user_config_path()?;
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
```

- [ ] **Step 2: Update app initialization to use the user-directory resolver**

```rust
let config = load_config_from_user_dir().map_err(|e| e.to_string())?;
```

- [ ] **Step 3: Run the targeted Rust tests to verify they pass**

Run: `cargo test resolves_config_path_from_user_local_app_data config_missing_returns_not_configured_state`

Expected: PASS.

### Task 3: Add Background Retry Tests

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing tests for retrying HA startup work three times**

```rust
    #[tokio::test]
    async fn startup_retry_helper_retries_three_times() {
        let attempts = Arc::new(StdMutex::new(0));
        let attempts_for_call = Arc::clone(&attempts);

        let result = retry_startup_task(3, || {
            let attempts_for_call = Arc::clone(&attempts_for_call);
            async move {
                let mut guard = attempts_for_call.lock().unwrap();
                *guard += 1;
                Err(anyhow!("temporary failure"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*attempts.lock().unwrap(), 3);
    }
```

- [ ] **Step 2: Run the targeted Rust test to verify it fails**

Run: `cargo test startup_retry_helper_retries_three_times`

Expected: FAIL because `retry_startup_task` does not exist yet.

### Task 4: Implement Background Retry Orchestration

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Add a retry helper and wire it into startup initialization**

```rust
async fn retry_startup_task<F, Fut>(max_attempts: usize, mut task: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut last_err = None;
    for _ in 0..max_attempts {
        match task().await {
            Ok(()) => return Ok(()),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("startup task failed")))
}
```

- [ ] **Step 2: Wrap HA startup actions in the retry helper without delaying window display**

```rust
let snapshot = retry_startup_task(3, || async {
    bootstrap_startup_mode(...).await?;
    fetch_current_snapshot(&config).await.map_err(|e| anyhow!(e))?;
    Ok(())
})
.await
```

- [ ] **Step 3: Ensure retries are background-only and fall back to offline snapshot**

```rust
tokio::spawn(async move {
    let _ = retry_startup_task(3, || async { ... }).await;
});
```

- [ ] **Step 4: Run the targeted Rust test to verify it passes**

Run: `cargo test startup_retry_helper_retries_three_times`

Expected: PASS.

### Task 5: Verify UI Still Builds And Shows Offline States

**Files:**
- Modify: `src/App.tsx`
- Test: `src/App.tsx`

- [ ] **Step 1: Add a failing test for showing offline/not-configured UI state**

```ts
// in src/App.tsx-related tests if existing test harness is unavailable, add a file-level regression test that asserts the status text logic produces a visible offline message when initError is set.
```

- [ ] **Step 2: Run the frontend build and typecheck**

Run: `npm run build && npm run lint`

Expected: PASS.

- [ ] **Step 3: Run the full Rust suite and Windows target check**

Run: `cargo test && cargo check --target x86_64-pc-windows-gnu`

Expected: PASS.

- [ ] **Step 4: Commit the implementation**

```bash
git add src-tauri/src/main.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src/App.tsx docs/superpowers/specs/2026-04-18-config-and-retry-separation-design.md docs/superpowers/plans/2026-04-18-config-and-retry-separation.md
git commit -m "feat: load config from user directory with background retries"
```
