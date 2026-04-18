# CI Windows Build Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the GitHub `Release` workflow by fixing the Windows build failures in Tauri without changing any CSS or UI component structure.

**Architecture:** Keep the fix tightly scoped to `src-tauri`. First add failing tests or failing verification for the two identified root causes: invalid icon asset and Windows API type mismatches. Then make the smallest code and asset changes needed to preserve the intended Windows shutdown-hook behavior while allowing the Tauri Windows build path to succeed.

**Tech Stack:** Rust, Tauri v1, `windows-sys`, Cargo unit tests, GitHub Actions Release workflow

---

## File Map

- Modify: `src-tauri/src/main.rs`
  - Contains the Windows-only shutdown-hook logic and the existing Rust unit tests.
- Modify: `src-tauri/icons/icon.ico`
  - Windows icon asset consumed by Tauri during build.
- Optional modify: `src-tauri/icons/icon.png`
  - Only if needed as the source for regenerating a valid `.ico` file.
- Create or modify: `docs/superpowers/plans/2026-04-17-ci-windows-build-fix.md`
  - This implementation plan.

### Task 1: Add a failing icon validity test

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add the following test inside the existing `#[cfg(test)] mod tests` block in `src-tauri/src/main.rs`:

```rust
    #[test]
    fn windows_icon_file_is_present_and_not_tiny_placeholder_data() {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
        let bytes = std::fs::read(&icon_path).expect("icon should exist");

        assert!(bytes.len() > 256, "icon should be a real ico file");
        assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "icon should have ico header");
    }
```

Also update the test imports near the top of the same module so `PathBuf` remains imported and `std::fs` can be called through its full path, with no other test structure changes required.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test windows_icon_file_is_present_and_not_tiny_placeholder_data -- --exact`

Expected: FAIL because the current `src-tauri/icons/icon.ico` file is only 90 bytes and does not satisfy the size assertion.

- [ ] **Step 3: Replace the broken icon with a valid `.ico` file**

Replace `src-tauri/icons/icon.ico` with a real ICO asset. Keep the path and filename exactly the same.

The replacement must satisfy all of the following:

```text
- It is a valid ICO container beginning with bytes 00 00 01 00.
- Its size is comfortably above 256 bytes.
- It is suitable for Windows app packaging.
- It does not require any Tauri config path changes.
```

If the safest route is regeneration from `src-tauri/icons/icon.png`, do that, but keep the frontend and visual structure untouched.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test windows_icon_file_is_present_and_not_tiny_placeholder_data -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/icons/icon.ico
git commit -m "test: validate windows icon asset"
```

### Task 2: Add a failing Windows-hook compatibility test

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add these tests inside the existing `#[cfg(test)] mod tests` block in `src-tauri/src/main.rs`:

```rust
    #[cfg(windows)]
    #[test]
    fn windows_hwnd_store_key_uses_pointer_value() {
        use super::windows_app::hwnd_store_key;

        let raw = 0x1234usize as *mut std::ffi::c_void;
        assert_eq!(hwnd_store_key(raw), 0x1234isize);
    }

    #[cfg(windows)]
    #[test]
    fn windows_query_end_session_result_is_truthy() {
        use super::windows_app::query_end_session_result;

        assert_eq!(query_end_session_result(), 1);
    }
```

This deliberately codifies the current `windows-sys` type shape used by CI: `HWND` is treated as a raw pointer alias and `LRESULT` is treated as a plain integer alias.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --target x86_64-pc-windows-gnu windows_hwnd_store_key_uses_pointer_value -- --exact`

Expected: FAIL before implementation because `hwnd_store_key` and `query_end_session_result` do not exist yet.

If the Windows GNU target is missing locally, install it first with:

Run: `rustup target add x86_64-pc-windows-gnu`

Expected: target installs successfully.

- [ ] **Step 3: Write minimal implementation**

First add the smallest helper functions needed to make the Windows handle/result behavior testable without changing UI behavior:

```rust
    fn hwnd_store_key(hwnd: HWND) -> isize {
        hwnd as isize
    }

    fn query_end_session_result() -> LRESULT {
        1
    }
```

Then update the Windows-only logic in `src-tauri/src/main.rs` to use the helpers and current `windows-sys` types correctly.

Apply these code changes inside `#[cfg(windows)] mod windows_app`:

```rust
    static ORIGINAL_WNDPROCS: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();

    fn wndproc_store() -> &'static Mutex<HashMap<isize, isize>> {
        ORIGINAL_WNDPROCS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn hwnd_store_key(hwnd: HWND) -> isize {
        hwnd as isize
    }

    fn query_end_session_result() -> LRESULT {
        1
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
            return query_end_session_result();
        }

        let key = hwnd_store_key(hwnd);
        let prev = wndproc_store()
            .lock()
            .ok()
            .and_then(|store| store.get(&key).copied());

        if msg == WM_NCDESTROY {
            if let Ok(mut store) = wndproc_store().lock() {
                store.remove(&key);
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
        let hwnd_key = hwnd_store_key(hwnd);
        unsafe {
            let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, main_window_proc as _);
            let mut store = wndproc_store().lock().map_err(|e| e.to_string())?;
            store.insert(hwnd_key, prev as isize);
        }
        Ok(())
    }

    pub fn handle_windows_message(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        if msg == WM_QUERYENDSESSION {
            let _ = (wparam, lparam);
            Some(query_end_session_result())
        } else {
            None
        }
    }
```

Keep all other behavior intact. Do not alter frontend code, CSS, or UI component structure.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --target x86_64-pc-windows-gnu windows_hwnd_store_key_uses_pointer_value -- --exact`

Expected: PASS.

Then run: `cargo test --target x86_64-pc-windows-gnu windows_query_end_session_result_is_truthy -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix: align windows tauri hook with windows-sys types"
```

### Task 3: Reproduce and clear the original build failure path

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Run the targeted failing verification first**

Run: `cargo check --target x86_64-pc-windows-gnu`

Expected before the full fix: either the same Windows type failures no longer appear because Task 2 is already green, or any remaining Windows-only incompatibility is surfaced clearly.

If Tauri build prerequisites block a full local Windows package build on Linux, this command is still the required low-cost compile-level verification.

- [ ] **Step 2: Run the focused local test suite**

Run: `cargo test`

Expected: PASS for all existing tests plus the new icon and Windows compatibility tests that are runnable on the current host.

- [ ] **Step 3: Run the strongest local build verification available**

Run: `npm run build`

Expected: PASS, confirming the frontend build remains unaffected.

Then run:

Run: `cargo check --target x86_64-pc-windows-gnu`

Expected: PASS, confirming the current CI compile failure mode has been removed.

- [ ] **Step 4: Inspect the diff for scope control**

Run: `git diff -- src-tauri/src/main.rs src-tauri/icons/icon.ico docs/superpowers/plans/2026-04-17-ci-windows-build-fix.md`

Expected: Only Windows build logic, tests, and icon asset changes are present. No CSS files or UI component structure changes appear.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/icons/icon.ico docs/superpowers/plans/2026-04-17-ci-windows-build-fix.md
git commit -m "fix: restore windows tauri release build"
```

## Self-Review Coverage

- Spec requirement: fix Windows Rust compile failures.
  - Covered by Task 2 and Task 3.
- Spec requirement: replace broken Windows icon resource.
  - Covered by Task 1 and Task 3.
- Spec requirement: TDD-first workflow.
  - Covered by Task 1 and Task 2 with explicit red/green steps.
- Spec requirement: no CSS or UI component structure changes.
  - Protected by task scope and Task 3 diff inspection.

No placeholders remain. No additional subsystems were introduced.
