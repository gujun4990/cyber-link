# Card-Only Window Design

**Goal:** Render only the center card UI in a frameless window, with no visible system title bar or outer shell.

**Architecture:** The Tauri window will be transparent and undecorated, sized to preserve the card's aspect ratio rather than matching a screenshot pixel-for-pixel. The React app will keep the original card internals, animations, and useful comments, while removing only the outer shell/background layers that produce visible perimeter space.

**Tech Stack:** Tauri 1, Rust, React, TypeScript, Tailwind CSS

---

## Requirements

- The visible UI must consist only of the center card.
- No system title bar, maximize, minimize, or close buttons may be visible.
- The card's inner visuals must remain intact, including:
  - top status bar
  - center temperature panel
  - side controls
  - bottom status area
  - existing motion effects, glow, scan, and ring animations
- Useful comments already present inside `App.tsx` should be preserved.
- The window should keep a fixed card aspect ratio and should not be locked to the exact pixel size of `f.jpg`.

## Design

### Window layer

- Use a frameless Tauri window (`decorations: false`).
- Keep the window transparent so there is no visible shell outside the card.
- Keep the window centered and sized so the card remains visually dominant.
- The custom window controls already inside the card remain the only controls.

### React layer

- Keep the original card content unchanged.
- Remove only the outer shell/background layers that create visible perimeter space.
- Preserve the card's existing dimensions, motion effects, and comments.
- Maintain the card's current layout so the screen still reads like the original dashboard card.

## Verification

- `node --test src/shellRemoval.test.js`
- `npm run build`
- `cargo test`
- `cargo check`

## Risks

- Transparent frameless windows can behave differently across Windows compositors and GPU drivers.
- Very small screen sizes may clip the fixed-aspect card.
