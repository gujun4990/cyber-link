# HA Shutdown Pending Automation

This feature delays the final Home Assistant shutdown sync for 30 seconds after Windows starts shutting down.

## Helpers

- `input_boolean.cyber_link_shutdown_pending`
- `timer.cyber_link_shutdown_delay`

## Flow

1. The Windows app sends `shutdown_pending` when it receives the shutdown notification.
2. Home Assistant turns on `input_boolean.cyber_link_shutdown_pending` and starts the 30-second timer.
3. If `startup_online` arrives before the timer finishes, Home Assistant cancels the timer and turns the boolean off.
4. If the timer finishes while the boolean is still on, Home Assistant runs the existing final shutdown sync.

## Notes

- The desktop app does not wait 30 seconds locally.
- The helper names above must match the automation and timer in Home Assistant exactly.
- The final shutdown action can reuse the existing shutdown-side logic already used by the app.
