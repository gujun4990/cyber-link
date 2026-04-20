# Packaging And Title Cleanup Design

**Goal:** standardize local Tauri commands and align the browser page title with the packaged app name so startup/debugging signals are less misleading.

**Scope:** small configuration-only cleanup. No Rust behavior changes, no UI behavior changes, no bundle logic changes beyond command wiring.

## Changes

- Add standard Tauri scripts to `package.json` so local development and release builds use a predictable entrypoint.
- Update `index.html` title from the template placeholder to the application name used by the desktop app.
- Keep all existing frontend and backend runtime code unchanged.

## Rationale

- The repo currently builds the frontend correctly but lacks standard Tauri npm scripts, which makes build and repro steps less obvious.
- The HTML title still uses the template name, which creates noisy signals during debugging and can be confused with runtime startup issues.
- Restricting this cleanup to config files keeps the change low risk.

## Verification

- `npm run build`
- Confirm the new npm scripts are present in `package.json`.
- Confirm `index.html` uses the application title instead of the template title.
