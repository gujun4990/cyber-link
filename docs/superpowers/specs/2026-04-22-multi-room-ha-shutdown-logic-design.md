# Multi-room HA Shutdown Logic Design

## Goal

Keep the Windows client responsible only for reporting its own online/offline state. Let Home Assistant handle per-room aggregation and the 30-second delayed shutdown for the last PC in each room.

## Problem

The current `input_boolean.cyber_link_shutdown_pending` helper is global. That works for a single shared automation, but it breaks room isolation because any startup can clear any pending shutdown.

## Proposed Design

### App responsibilities

- Send `pc_entity_id = on` when the app starts.
- Send `pc_entity_id = off` when Windows actually shuts down.
- Do not send a global `shutdown_pending` signal.
- Do not keep any cross-room shutdown timer state in the app.

### Home Assistant responsibilities

- Aggregate the `pc_entity_id` entities for each room into a room-level online sensor.
- Use one automation or blueprint instance per room.
- When any PC in a room turns on, turn on that room's AC and configured lights.
- When the room-level online sensor stays `off` for 30 seconds, turn off that room's AC and configured lights.

## Data Model

### `pc_entity_id`

`pc_entity_id` remains the only entity the app needs for online/offline reporting.

### Room aggregation

Each room gets its own HA aggregate entity, such as:

- `binary_sensor.room1_any_pc_online`
- `binary_sensor.room2_any_pc_online`

These are derived only from the PCs assigned to that room.

## Behavior

### Startup

1. App starts.
2. App marks its own `pc_entity_id` as `on`.
3. HA turns on the room's AC and lights immediately.

### Shutdown

1. Windows actually shuts down.
2. App marks its own `pc_entity_id` as `off`.
3. HA recomputes the room aggregate.
4. If the room still has at least one online PC, nothing turns off.
5. If the room has no online PCs for 30 seconds, HA turns off the room's AC and lights.

## Non-goals

- No global shutdown helper entity.
- No room-sharing or cross-room cancellation logic in the app.
- No change to how optional device entities are configured on a per-PC basis.

## Migration Notes

- Remove references to `input_boolean.cyber_link_shutdown_pending` from the app and setup docs.
- Keep the room-level HA automation documentation as the source of truth for the 30-second delay.

## Success Criteria

- A PC in room A does not affect shutdown timing in room B.
- App code only reports local machine state.
- The 30-second delay exists only in HA room automations.
