# Config And Retry Separation Design

**Goal:** keep the UI visible even when Home Assistant is unavailable, load configuration from the user profile directory only, create the app-local directory on startup, and retry HA startup actions in the background without blocking app launch.

**Scope:** configuration lookup, startup initialization retry behavior, and error presentation. No automatic config file creation and no UI redesign.

## User-Facing Behavior

- The main window appears immediately on manual launch.
- The app stays visible even if `config.json` is missing.
- The app shows a clear “not configured” or offline state when config is missing or HA is unreachable.
- HA-related startup work retries in the background up to 3 times.
- Startup retries do not block the UI from rendering.

## Configuration Location

Read `config.json` from the user profile directory only:

- Windows: `%LOCALAPPDATA%/cyber-link/config.json`

The app should create `%LOCALAPPDATA%/cyber-link/` on startup, but it should not auto-create `config.json` on first run. If the file is missing, the UI should surface that fact instead of silently generating a template.

## Retry Policy

Apply retry only to HA startup operations, not to window display.

- Retry count: 3 attempts
- Retry scope:
  - fetch current HA snapshot during app initialization
  - startup online notification on autostart
  - autostart-only device activation flow
- Retry behavior:
  - retries happen in the background
  - UI rendering continues immediately
  - once retries are exhausted, the app falls back to the existing offline snapshot/state path

This keeps launch responsive while still handling transient HA/network issues.

## Architecture

### Configuration loading

Add a config resolver that reads from the user directory path only. The resolver should be the single source of truth for startup and action commands so both manual launch and autostart use the same config path behavior.

### Startup orchestration

Refactor initialization into two independent steps:

1. show/focus the window immediately according to startup mode
2. start background HA initialization with retry

The second step should not gate the first. If config is missing, the app should still render and the background task should update the UI into a clear disconnected/misconfigured state.

### Error handling

- Missing config: show a deterministic “not configured” state
- HA request failure: retry up to 3 times in the background
- Retry exhaustion: fall back to offline snapshot without closing or hiding the UI

## File-Level Changes

### `src-tauri/src/main.rs`

- Update config path resolution to user directory only.
- Add a retry helper for HA startup operations.
- Split window presentation from HA initialization.
- Keep the existing startup-mode split and single-instance behavior.

### `src/App.tsx`

- Keep first render independent of `initialize_app` success.
- Show missing-config and offline states when initialization reports failure.

### Tests in `src-tauri/src/main.rs`

- config path resolver returns the user directory path
- missing config does not prevent window presentation logic from running
- HA startup retry helper retries 3 times before falling back
- startup failure after retries still emits an offline snapshot

## Testing Strategy

- `cargo test`
- `cargo check --target x86_64-pc-windows-gnu`
- `npm run build`
- `npm run lint`

## Non-Goals

- No automatic config file creation
- No retry for window display or single-instance activation
- No UI redesign
