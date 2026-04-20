# Fixed Size, Drag, and Offline State Design

**Goal:** Keep the interface fixed-size, make the top bar draggable without double-click side effects, and ensure startup/offline renders the full UI in an off state.

**Architecture:** Keep the single Tauri window and the existing `windowSize.json` as the single size source. Replace system drag-region behavior with explicit top-bar dragging so double-click can be ignored, make the Windows app a GUI binary so no console window appears, and derive all startup/offline visual state from a separate display flag so every effect that implies “active” stays off until a real HA snapshot is known.

**Tech Stack:** React, Tauri 1, Rust, TypeScript, Node test runner

---

### Drag Behavior

The top bar must remain draggable with the left mouse button, but double-click must not maximize, restore, or otherwise change window behavior. The implementation should use an explicit mouse handler on the top bar instead of `data-tauri-drag-region`.

Only the top bar’s empty area participates in dragging. Buttons and other interactive controls inside the header remain clickable and do not start a drag.

**Looks right so far?**

### Fixed Size

The interface size stays fixed at the current `windowSize.json` value. The React card and the Tauri window must both use the same width and height, and the window must not be resizable or maximizable from any edge.

The Tauri configuration should disable resizing, and the Rust side should continue to force the window size to the shared values when the window is shown or restored.

**Looks right so far?**

### Startup and Offline Visual State

When the app is initializing, missing config, or HA connection fails, the whole interface must render in an off state. That includes:
- the AC switch
- the light switch
- cooling/heating mode badges
- the temperature number and glow treatment

Availability still controls whether the buttons can be clicked. Display state is separate: loading/offline must look off even if the last known snapshot or defaults would otherwise look on.

The visual state should only show active styling when `hasLoadedState` is true and the app is connected.

**Looks right so far?**

### Testing

- Verify the top bar uses explicit drag handling and ignores double-click
- Verify the Tauri window is non-resizable
- Verify the shared window size is still used by both sides
- Verify startup/offline display state keeps AC/light and related visuals off
- Keep existing tray and restore behavior tests passing

### Non-Goals

- No redesign of the card layout
- No change to the HA protocol or config file format
- No change to tray menu behavior
