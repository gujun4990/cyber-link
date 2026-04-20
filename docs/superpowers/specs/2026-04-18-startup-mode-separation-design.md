# Startup Mode Separation Design

**Goal:** split manual launch and Windows autostart into two explicit startup modes so double-click launch only opens the UI and fetches Home Assistant state, while boot autostart runs in the tray and performs the startup automation.

**Scope:** Rust startup mode detection, autostart registry value format, window visibility on launch, startup initialization branching, and regression coverage. No Tauri version change and no new dependencies.

## User-Facing Behavior

### Manual launch

- User double-clicks the executable.
- The main window opens immediately.
- The app fetches the current Home Assistant state.
- The app does **not** auto-turn-on the PC online boolean, air conditioner, or light.

### Windows autostart launch

- Windows launches the app from the user Run registry key.
- The app starts hidden and only shows the tray icon.
- The app performs startup automation:
  - mark the PC entity online
  - turn on air conditioning when configured
  - turn on the light when configured
- The user can open the main window later from the tray menu.

## Recommended Approach

Use an explicit command-line flag to identify autostart launches.

- Manual launch keeps the normal executable invocation with no extra arguments.
- The autostart registry entry becomes `"<exe path>" --autostart`.
- The Rust backend checks for `--autostart` during process startup.

This is the most stable option because it does not rely on timing, environment variables, or temporary files. The launch source is explicit and testable.

## Architecture

### Startup mode detection

Introduce a small Rust startup mode concept with two values:

- `Manual`
- `Autostart`

The mode is derived from `std::env::args()`.

### Registry entry format

Update the autostart registry value helper so the saved Run value matches the autostart mode invocation:

- current: `"<exe path>"`
- new: `"<exe path>" --autostart`

`verify_autostart_registry_entry()` must validate against the same new string.

### Initialization split

Refactor the startup path so startup automation is conditional on startup mode:

- `Manual`
  - load config
  - fetch current snapshot from Home Assistant
  - if snapshot fetch fails, fall back to offline snapshot as today

- `Autostart`
  - attempt autostart registry registration/verification with the existing access-denied tolerance
  - run `send_startup_online(config)`
  - fetch current snapshot from Home Assistant
  - if startup automation or snapshot fetch fails, fall back to offline snapshot as today

This keeps the backend as the single source of truth for launch behavior and avoids duplicating mode logic in React.

### Window visibility and tray behavior

Startup mode also controls the initial window presentation:

- `Manual`
  - main window is shown
- `Autostart`
  - main window remains hidden
  - tray remains active

The existing tray menu can continue to expose `打开` and `退出` without structural changes.

## File-Level Changes

### `src-tauri/src/main.rs`

- Add startup mode parsing from process arguments.
- Update autostart registry value generation to include `--autostart`.
- Branch initialization behavior based on startup mode.
- Use startup mode to decide whether to hide the main window during boot.
- Preserve the current access-denied tolerance for locked-down Windows environments.

### `src/App.tsx`

- No startup mode branching should be added to the frontend.
- The frontend continues to call `initialize_app` and render the returned snapshot.

### Tests in `src-tauri/src/main.rs`

Add or update tests for:

- autostart registry value includes `--autostart`
- manual launch does not run startup automation
- autostart launch runs startup automation after autostart setup
- access-denied autostart registry failures are still tolerated

## Error Handling

- Manual launch should not be treated as a startup automation failure path because it no longer performs startup automation.
- Autostart launch continues to tolerate Windows registry access-denied errors during autostart setup.
- Home Assistant request failures should still degrade to the existing offline snapshot behavior rather than crashing startup.

## Testing Strategy

### Rust unit tests

- verify startup mode parsing
- verify autostart registry value contents
- verify manual startup skips `send_startup_online`
- verify autostart startup performs setup before automation
- verify access-denied tolerance still works

### Existing verification

- `cargo test`
- `npm run build`
- `npm run lint`

## Non-Goals

- No redesign of the tray menu
- No changes to Home Assistant API shapes
- No Tauri upgrade
- No new crates or npm packages
