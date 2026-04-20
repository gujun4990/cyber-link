# Single Visible Surface Design

**Goal:** when the app is launched, only the centered main control surface should be visible; the black shell window shown in `x.png` must not appear as a separate user-facing surface.

**Scope:** Tauri window presentation, startup visibility, and React root layout. No changes to Home Assistant behavior or refresh semantics.

## Observed Behavior

- `w.jpeg` shows the desired centered control surface.
- `x.png` shows an unwanted black window/shell.
- The desired end state is one user-facing surface only: the centered UI from `w.jpeg`.

## Design Direction

Use a single visible Tauri window and make the React app render only the centered control card as the primary visual content. The startup flow should not create any extra visible shell or duplicate window.

This design treats the black screen as an implementation artifact, not a product surface.

## Architecture

### Window layer

- Keep one `main` window only.
- Manual launch should show that one window immediately.
- Autostart may still launch hidden and later reveal the same window.
- Second launches should focus the existing window rather than creating another visible one.

### UI layer

- The React root should render the centered control card as the primary visible content.
- Any full-screen dark background should be treated as non-product chrome and minimized or removed if it causes the black-shell look.
- The visible surface should match `w.jpeg`.

### Startup behavior

- Window show/hide behavior must not create a separate blank shell.
- The single-instance handling should only activate the existing window.
- UI initialization should remain asynchronous so the first paint is not blocked.

## Implementation Options

### Option 1: Remove the black shell from the React root

- Keep one window.
- Keep the centered control card.
- Remove or greatly reduce the full-screen black background container that produces the shell appearance.

Tradeoff: fastest way to match `w.jpeg`, but the current layout must be adjusted carefully to preserve spacing.

### Option 2: Keep the shell but make it invisible/transparent

- Keep one window.
- Make the outer shell transparent or visually neutral.
- Leave the centered card unchanged.

Tradeoff: lower UI churn, but can still leave behind layout oddities if the shell is the main source of the black-screen look.

### Option 3: Eliminate any extra visible window behavior at the Tauri layer

- Ensure there is only one `main` window visible at any point.
- Any second launch just focuses the existing window.
- Combine with a minimal root layout cleanup if needed.

Tradeoff: best long-term structure, but if the black screen is purely a layout issue, this alone will not be enough.

**Recommended approach:** Option 1 plus the single-window guard from Option 3 if needed. That gives the most direct path to matching `w.jpeg` while ensuring no duplicate visible window survives startup.

## Error Handling

- If the app cannot load config or reach Home Assistant, it should still render the centered control surface.
- The user should see a status message inside the visible surface instead of a separate black shell.
- Startup failures must not create an extra visible blank window.

## Testing Strategy

- Verify only one visible app surface appears on launch.
- Verify manual launch shows the centered surface immediately.
- Verify autostart still reuses the existing window.
- Verify the visible UI matches the centered control card layout.

## Non-Goals

- No change to the Home Assistant command set.
- No change to refresh behavior.
- No additional windows or splash screens.
