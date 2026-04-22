# Windows Shutdown Pending Design

**Goal:** When Windows is actually shutting down, the app should send a shutdown intent immediately, and Home Assistant should wait 30 seconds before applying the final offline/shutdown action.

**Architecture:** Keep the current Tauri desktop app as the source of truth for shutdown intent, but move the 30-second delay and cancel logic into Home Assistant. The app emits a lightweight `shutdown_pending` signal during `WM_QUERYENDSESSION`; HA starts a timer, cancels it if `startup_online` arrives again, and only performs the final shutdown sync if the timer expires while the machine is still offline.

**Tech Stack:** Rust, React, Tauri 1, TypeScript, Home Assistant automations/timer helpers

---

### Shutdown Flow

When Windows begins shutting down, the desktop app must not wait 30 seconds locally.

Instead, it sends a `shutdown_pending` intent to HA as soon as the shutdown notification is received.

HA then owns the delay window:

- start a 30-second timer
- mark the shutdown as pending
- if `startup_online` arrives before the timer finishes, cancel the pending shutdown
- if the timer finishes while pending is still active, run the final shutdown sync

The final shutdown sync should reuse the existing HA-side control behavior already used by `shutdown_signal`.

**Looks right so far?**

### HA State Model

Add one HA-side pending marker and one timer helper:

- `input_boolean.cyber_link_shutdown_pending`
- `timer.cyber_link_shutdown_delay`

The pending marker records whether a shutdown intent is waiting for confirmation.
The timer enforces the 30-second delay without relying on the Windows process staying alive.

`startup_online` remains the cancellation signal:

- if Windows comes back online before the timer finishes, clear the pending marker and cancel the timer
- if Windows stays offline, let the timer expire and finalize the shutdown state

The final shutdown action will reuse the existing shutdown-side HA control path instead of introducing a second finalization contract.

**Looks right so far?**

### App Responsibilities

The desktop app only needs to:

- detect the Windows shutdown notification
- send `shutdown_pending` once
- keep the existing startup/reconnect path unchanged

The app should not sleep for 30 seconds locally, and it should not try to guarantee the final shutdown action by itself.

If HA is unavailable when the intent is sent, the app should log the failure and leave the existing local shutdown path unchanged.

**Looks right so far?**

### Non-Goals

- No new Windows Service
- No local 30-second timer in the GUI app
- No change to the current `startup_online` startup flow
- No redesign of the existing lighting/AC control paths

### Testing

- Add a Tauri/Rust test that confirms the Windows shutdown path emits `shutdown_pending` rather than delaying locally.
- Add a Rust test for the new shutdown-intent command/action shape.
- Add a doc-level verification note for the HA automation/timer sequence.

### Open Questions

None. The design intentionally uses both `timer` and `input_boolean`, and the final shutdown action reuses the existing shutdown-side control path.
