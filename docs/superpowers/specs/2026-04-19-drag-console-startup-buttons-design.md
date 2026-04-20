# Drag, Console, and Startup Buttons Design

**Goal:** Make the top bar draggable without double-click side effects, remove the extra black console window on Windows, and ensure AC/light buttons start in the off state until real HA state is known.

**Architecture:** Keep the existing single-window Tauri app, but move drag handling from the system drag region to an explicit top-bar mouse handler so double-click can be ignored. Remove the Windows console subsystem output by building the Rust binary as a GUI app. Split button display state from backend truth so startup/loading/offline always renders both switches off until HA state arrives.

**Tech Stack:** React, Tauri 1, Rust, TypeScript, Node test runner

---

### 1. Drag-Only Top Bar

The top bar should still let the user move the window by left-dragging, but double-click should do nothing. The implementation should not rely on `data-tauri-drag-region`, because that reintroduces Windows title-bar double-click semantics. Instead, the top bar will listen for a left-button press and invoke the Tauri window drag API from an explicit handler. The double-click event on the same region will call `preventDefault()` and `stopPropagation()` so it cannot trigger maximize/restore behavior.

This change stays scoped to the header row in `src/App.tsx`. Buttons inside the header remain clickable and must not begin a drag. The body and footer remain unchanged.

**Looks right so far?**

### 2. Remove The Extra Black Window

The screenshot `k.png` shows a Windows console window, not the app content itself. The fix is to compile the Rust executable as a GUI subsystem binary so Windows does not create a console host window for the release app. The Tauri webview window remains unchanged.

This should be a small Rust-side change near the crate entrypoint, plus a regression test that checks for the GUI subsystem attribute or equivalent Windows-specific build marker.

**Looks right so far?**

### 3. Startup Button Off State

On first launch, during initialization, or when HA/config loading fails, both `空调核心系统` and `环境氛围照明` should visually render as off. The buttons may still be disabled when unavailable, but their active styling must not appear until a real snapshot confirms the state.

The simplest approach is to derive a separate display flag from `hasLoadedState`, `initFailed`, and `device.connected` rather than using `device.ac.isOn` / `device.lightOn` directly for styling. In other words, availability controls whether they can be clicked; startup state controls whether they look on.

This keeps the offline/loading experience predictable:
- loading or offline -> both switches look off
- live HA snapshot -> switches reflect real state

**Looks right so far?**

### Testing

- Verify the top bar no longer uses `data-tauri-drag-region`
- Verify the top bar has an explicit drag handler and ignores double-click
- Verify the Windows GUI subsystem marker is present so no console window is spawned
- Verify startup/offline state renders both switches as off
- Keep existing window-size and tray tests passing

### Non-Goals

- No redesign of the card content
- No change to tray behavior
- No change to HA protocol or config loading semantics
