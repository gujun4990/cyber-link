# HA Room Shutdown Delay

This feature delays the final room shutdown by 30 seconds after the last PC in that room goes offline.

## Inputs

- Required: every PC in the room must expose a `pc_entity_id`.
- `binary_sensor.<room>_any_pc_online`
- room-specific AC entity
- room-specific light entities

## Flow

1. The Windows app sends only its own `pc_entity_id` on startup and shutdown.
2. Home Assistant aggregates the PCs for each room into a room-level online sensor.
3. If any PC in the room turns on, HA turns on that room's AC and configured lights.
4. If the room-level sensor stays `off` for 30 seconds, HA turns off that room's AC and configured lights.

## Notes

- The desktop app does not keep any room-level shutdown state.
- The 30-second delay lives only in Home Assistant.
- Each room needs its own automation or blueprint instance.
