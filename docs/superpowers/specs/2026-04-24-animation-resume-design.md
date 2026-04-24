# Animation Resume Design

**Goal:** When the window is restored after being minimized or hidden, the UI animations should start again.

**Architecture:** Keep the existing pause behavior for hidden windows, but add a separate resume trigger that advances an animation restart token when the window becomes visible again. Key animated regions will use that token to remount and replay their animations once on restore.

**Tech Stack:** React, TypeScript, Motion, Tauri

---

## Requirements

- While the window is hidden or minimized, animation work should stay paused.
- When the window is shown again, animations should replay automatically.
- The resume behavior should work for both tray reopen and taskbar/window restore.
- The change should stay local to the UI state flow and not affect Home Assistant commands.

## Design

### Pause state

- Keep using the existing visibility-based pause flag to suppress animation work while hidden.
- Do not change the current pause-on-hide behavior.

### Resume trigger

- Add a small restart token in `App.tsx`.
- Increment that token when the window transitions from paused to visible.
- Use the token as a `key` or equivalent restart signal for the main animated surface so motion components remount and replay.

### Scope

- Apply the restart token to the UI regions that currently depend on `pauseAnimations`.
- Leave data refresh, HA actions, and device state logic unchanged.

## Verification

- `npm run lint`
- `cargo test`
- Manual check: minimize the window, restore it, and confirm the main ring/fan/status animations restart.

## Risks

- Remounting animated regions may briefly reset local animation phase on restore, which is expected.
- If the restart token is applied too broadly, it could reset unrelated UI state.
