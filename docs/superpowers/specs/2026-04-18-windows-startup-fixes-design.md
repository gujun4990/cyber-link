## Summary

Fix two Windows post-install startup issues without changing CSS or UI component structure:

1. Remove the `Asset favicon.ico not found; fallback to favicon.ico.html` startup error by ensuring the frontend bundle contains a real `favicon.ico` asset and uses desktop-safe asset paths.
2. Prevent the Windows release build from opening a console window during normal GUI startup.

## Constraints

- Do not modify any CSS.
- Do not change UI component structure.
- Keep the fix minimal and limited to startup, packaging, and static asset handling.
- Preserve console visibility in debug builds to avoid hurting local development.

## Current State

- The app is a Tauri 1 desktop application with a Vite-built React frontend.
- `dist/index.html` currently references built assets with absolute paths like `/assets/...`.
- The project has Tauri bundle icons under `src-tauri/icons/`, but the frontend bundle does not currently provide a `favicon.ico` file.
- The Rust entry point does not currently opt into the Windows GUI subsystem, so Windows launches the app with an attached console window.

## Root Cause

### Missing favicon asset

At startup, the webview requests `favicon.ico`. The packaged frontend assets do not include that file, so Tauri logs the missing asset and falls back to `favicon.ico.html`.

### Console window on Windows

The Windows binary is built as a console subsystem application. That is appropriate for debugging, but for an installed tray-style GUI app it causes an unwanted black console window to appear during startup.

## Chosen Approach

Use a three-part fix:

1. Add a real frontend `favicon.ico` file using the existing Tauri icon asset as the source.
2. Configure Vite to emit relative asset URLs so the packaged desktop app resolves frontend assets consistently from the local bundle instead of relying on root-relative paths.
3. Mark Windows non-debug builds as GUI subsystem binaries so release startup does not open a console window.

## Alternatives Considered

### Only add `favicon.ico`

This addresses the visible asset error, but leaves the build using root-relative frontend asset paths, which is less robust in desktop packaging.

### Only suppress the console window

This hides one symptom from the user but leaves the missing asset warning unfixed.

### Change frontend markup or component structure

Rejected because the user explicitly requested no CSS or UI structure changes, and the issue does not require them.

## Detailed Changes

### 1. Frontend static favicon

- Add `public/favicon.ico` so Vite copies it directly into the build output.
- Reuse the existing icon from `src-tauri/icons/icon.ico` to avoid introducing a second independent icon source.

Expected result:

- `dist/favicon.ico` exists after build.
- The webview request for `favicon.ico` resolves successfully.

### 2. Relative asset paths for desktop bundle

- Update `vite.config.ts` to set `base: './'`.

Expected result:

- Built JS and CSS references in `dist/index.html` become relative, for example `./assets/...`.
- The packaged desktop app resolves bundled assets reliably from the local app content.

### 3. Windows GUI subsystem for release builds

- Add a crate-level attribute in `src-tauri/src/main.rs`:

```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
```

Expected result:

- Windows release builds launch without a console window.
- Debug builds still keep console output for local troubleshooting.

## Verification Plan

### Linux-side packaging checks

Run local build commands to confirm the file and path changes are present:

1. `npm run build`
2. Inspect `dist/index.html`
3. Confirm `dist/favicon.ico` exists
4. Run Rust tests if needed to ensure no entry-point regressions were introduced

Expected verification:

- `dist/index.html` uses relative asset URLs
- `dist/favicon.ico` is present

### Windows behavior checks

Run a Windows build and launch the packaged app locally.

Expected verification:

- Startup does not show `Asset favicon.ico not found; fallback to favicon.ico.html`
- Startup does not open a black console window
- Tray app behavior remains unchanged

## Scope Boundaries

Included:

- Static asset packaging
- Vite asset base configuration
- Windows binary subsystem configuration
- Local verification for the startup path

Excluded:

- Any CSS changes
- Any React component tree changes
- Any UI redesign or behavior changes unrelated to startup packaging

## Risks

- Setting `base: './'` changes how built assets are referenced. This is low risk for a packaged desktop app and should improve portability, but it must be verified against the existing Tauri startup flow.
- Reusing the Tauri icon as the frontend favicon means both surfaces share the same source asset, which is desirable here because the app already ships that icon.

## Rollback

If the relative asset path change causes an unexpected regression, roll back `base: './'` and keep the favicon plus Windows subsystem fix isolated. That said, the expected desktop-safe configuration is to use relative paths for packaged local assets.
