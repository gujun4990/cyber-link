# Air Controller Style HA Refactor Design

**Goal:** refactor the Rust backend toward the `air-controller` style of separation of concerns while keeping this app's full feature set: air conditioner control, light control, PC online/offline signaling, and snapshot-based UI state.

**Scope:** backend Rust structure, Home Assistant client flow, state snapshot assembly, action confirmation behavior, and regression coverage. No frontend redesign and no protocol change for existing Tauri commands.

## User-Facing Behavior

- Air conditioner on/off remains available.
- Air conditioner temperature setting remains available.
- Light on/off remains available.
- PC online/offline signaling remains available.
- Action results should reflect confirmed state whenever HA can be queried successfully.
- Temperature handling should be more robust than a raw pass-through:
  - read current HA climate state first
  - infer or respect temperature units
  - clamp and normalize to entity limits and step size when available

## Recommended Approach

Use a small client/service split, similar to `air-controller`:

- `main.rs` becomes startup and Tauri wiring only.
- `commands.rs` becomes the command facade.
- `ha_client.rs` owns HTTP requests, service calls, and HA state parsing.
- `snapshot.rs` owns conversion from HA entity states into the UI snapshot.
- `models.rs` keeps config and response types.

This keeps the code easy to reason about without introducing a generic abstraction layer that would be overkill for just climate and light control.

## Architecture

### Home Assistant client

Introduce a backend client that owns:

- authenticated `reqwest::Client`
- `get_state` for climate and light entities
- `turn_on` / `turn_off` for climate and light services
- `set_temperature` for climate
- error mapping for HTTP failures and missing config

Climate actions should follow the `air-controller` model:

- read entity state before temperature changes
- detect temperature unit from entity attributes when possible
- convert between Fahrenheit and Celsius when needed
- clamp to `min_temp` / `max_temp`
- normalize to `target_temp_step` when present
- confirm the post-action state by polling until the expected state is observed or retries are exhausted

Light actions should use the same request/confirm pattern, but with simple on/off semantics only.

### Snapshot assembly

Move all “turn HA entity JSON into app state” code into a dedicated snapshot module.

Snapshot rules:

- climate snapshot includes on/off state and temperature
- light snapshot includes on/off state
- PC entity remains part of the snapshot for online/offline signaling
- missing optional entities should not break snapshot creation
- disconnected or partially configured states should degrade predictably

### Command facade

Keep Tauri commands thin:

- validate config/token presence
- call the client
- return `ServiceResult<T>` or the current equivalent app result type
- do not embed HA request construction inside command handlers

## File-Level Changes

### `src-tauri/src/main.rs`

- remove direct HA request construction and snapshot parsing from the top-level file
- keep app bootstrap, tray, window, and startup code
- delegate backend actions to the new modules

### `src-tauri/src/commands.rs`

- expose the current commands through a thin facade
- add explicit light command helpers if needed by the UI layer
- keep startup auto-power-on orchestration intact

### `src-tauri/src/ha_client.rs`

- implement climate and light service calls
- add climate temperature normalization logic
- add polling-based confirmation for action completion
- preserve detailed HTTP error messages

### `src-tauri/src/snapshot.rs`

- parse HA entity state into the app snapshot
- keep partial availability handling
- preserve existing PC/light/climate merge behavior

### `src-tauri/src/models.rs`

- keep `AppConfig`, `ClimateState`, `ServiceResult`, and startup store types
- add only the minimal extra types needed for the split

## Error Handling

- Missing AC or light entity IDs should skip that control path cleanly.
- Missing token or invalid config should fail before the request is sent.
- Climate temperature changes should surface parse/conversion failures clearly.
- If polling never confirms the new state, return the latest observed state with a useful message.
- If HA is unreachable, keep the previous behavior of surfacing a failure instead of fabricating success.

## Testing Strategy

- client tests for climate turn on/off request URLs and bodies
- client tests for light turn on/off request URLs and bodies
- climate temperature normalization tests for:
  - unit inference
  - min/max clamping
  - step rounding
- snapshot tests for partial and missing entities
- action confirmation tests for success and timeout paths
- command tests to ensure the facade still returns the expected app result shape

## Non-Goals

- No generic “control any Home Assistant entity” abstraction
- No UI redesign
- No new authentication flow
- No behavior change for tray/window startup beyond what is needed to preserve the existing app
