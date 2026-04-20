# Autostart, Drag, Fixed Size, and Offline-Off Design

**Goal:** On Windows autostart, bring the app up hidden and turn on AC/light only after HA is reachable, while keeping the UI fixed-size and showing the full interface in an off state until real data is known.

**Architecture:** Keep the existing single Tauri window and shared `windowSize.json` as the size source. Use explicit top-bar drag handling so double-click can be ignored, make the Windows binary a GUI app so no console window appears, and split startup display state from device truth so loading/offline always looks off. Autostart mode will reuse the current startup bootstrap flow but add a post-connect power-on step only when the app launched with `--autostart`.

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

### Autostart Power-On

When the app is launched in `--autostart` mode, it should try to bring up the HA-backed power state after the app is visible and connected. If HA config is missing or HA connection fails, the app must not turn devices on. In that case it stays hidden/offline-looking and the UI remains off.

Manual launches do not auto-power-on devices.

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
- Verify the Tauri window is non-resizable and non-maximizable
- Verify autostart mode performs the power-on flow only in autostart mode
- Verify startup/offline display state keeps AC/light and related visuals off
- Keep existing tray, restore, and startup tests passing

### Non-Goals

- No redesign of the card layout
- No change to the HA protocol or config file format
- No change to tray menu behavior
