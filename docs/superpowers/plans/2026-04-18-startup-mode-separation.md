# Startup Mode Separation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** split manual launch and Windows autostart so manual launch only opens the UI and fetches HA state, while autostart stays hidden in the tray and runs startup automation.

**Architecture:** Add an explicit Rust startup mode parsed from process arguments and encode autostart launches as `--autostart` in the Windows Run registry value. Keep the frontend calling the same `initialize_app` command while the backend decides whether to run startup automation and whether the main window should stay hidden.

**Tech Stack:** Rust, Tauri v1, React, Node test runner, Cargo tests

---

### Task 1: Add Startup Mode Tests

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing tests for autostart argument handling and manual startup behavior**

```rust
    #[test]
    fn parses_autostart_mode_from_args() {
        assert!(matches!(startup_mode_from_args(["app.exe", "--autostart"]), StartupMode::Autostart));
        assert!(matches!(startup_mode_from_args(["app.exe"]), StartupMode::Manual));
    }

    #[test]
    fn autostart_registry_value_includes_autostart_argument() {
        let value = autostart_registry_value(
            Path::new(r"C:\Program Files\cyber-link\cyber-link.exe"),
        );

        assert_eq!(
            value,
            "\"C:\\Program Files\\cyber-link\\cyber-link.exe\" --autostart",
        );
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
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail for the expected reasons**

Run: `cargo test parses_autostart_mode_from_args autostart_registry_value_includes_autostart_argument manual_startup_skips_boot_automation`

Expected: FAIL because `StartupMode` and `bootstrap_startup_mode` do not exist yet and the registry value still omits `--autostart`.

### Task 2: Implement Backend Startup Mode Separation

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Add startup mode parsing and split startup automation by mode**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupMode {
    Manual,
    Autostart,
}

fn startup_mode_from_args<I, S>(args: I) -> StartupMode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if args.into_iter().skip(1).any(|arg| arg.as_ref() == "--autostart") {
        StartupMode::Autostart
    } else {
        StartupMode::Manual
    }
}

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
        StartupMode::Autostart => {
            enable_autostart()?;
            startup_online().await?;
            Ok(())
        }
    }
}
```

- [ ] **Step 2: Update the registry value helper to append the autostart flag**

```rust
pub fn autostart_registry_value(exe_path: &Path) -> String {
    format!("\"{}\" --autostart", exe_path.display())
}
```

- [ ] **Step 3: Use the parsed startup mode inside `initialize_app`**

```rust
        let startup_mode = startup_mode_from_args(std::env::args());
        let snapshot = match bootstrap_startup_mode(
            startup_mode,
            || {
                // existing registry setup with access-denied tolerance
            },
            || send_startup_online(&config),
        )
        .await
```

- [ ] **Step 4: Hide the window on setup only for autostart launches**

```rust
            .setup(|app| {
                if matches!(startup_mode_from_args(std::env::args()), StartupMode::Autostart) {
                    hide_main_window(app);
                }
                Ok(())
            })
```

- [ ] **Step 5: Run the targeted Rust tests to verify they now pass**

Run: `cargo test parses_autostart_mode_from_args autostart_registry_value_includes_autostart_argument manual_startup_skips_boot_automation`

Expected: PASS.

### Task 3: Preserve Existing Startup Failure Semantics

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Add a failing test that autostart still runs setup before startup automation**

```rust
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
```

- [ ] **Step 2: Keep the current access-denied tolerance inside the autostart-only setup branch**

```rust
                if let Err(err) = write_autostart_registry_entry(true) {
                    if tolerate_autostart_error(&err) {
                        eprintln!("autostart enable skipped: {err}");
                        return Ok(());
                    }
                    return Err(anyhow!(err));
                }
```

- [ ] **Step 3: Run the full Rust suite**

Run: `cargo test`

Expected: all Rust tests pass.

### Task 4: Verify Frontend And Project Checks

**Files:**
- Test: `src/App.tsx`
- Test: `package.json`

- [ ] **Step 1: Build the frontend with the backend changes in place**

Run: `npm run build`

Expected: Vite build completes successfully.

- [ ] **Step 2: Run the TypeScript check**

Run: `npm run lint`

Expected: `tsc --noEmit` exits successfully.

- [ ] **Step 3: Commit the implementation**

```bash
git add src-tauri/src/main.rs package.json src-tauri/tauri.conf.json docs/superpowers/specs/2026-04-18-startup-mode-separation-design.md docs/superpowers/plans/2026-04-18-startup-mode-separation.md
git commit -m "feat: separate manual launch and autostart"
```
