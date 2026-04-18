# Windows Startup Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the missing `favicon.ico` startup warning and stop the Windows release app from opening a console window, without changing CSS or UI component structure.

**Architecture:** Keep the fix limited to packaging and startup configuration. Add a real frontend favicon through Vite's static `public/` pipeline, switch built asset URLs to relative paths for desktop packaging, and mark Windows non-debug builds as GUI subsystem binaries so installed release builds launch without a console window.

**Tech Stack:** Vite 6, React 19, Tauri 1, Rust 2021

---

## File Map

- Create: `public/favicon.ico`
  Responsibility: ship a real favicon into the frontend build output as `dist/favicon.ico`.
- Modify: `vite.config.ts`
  Responsibility: emit relative asset URLs that are safe for packaged desktop startup.
- Modify: `src-tauri/src/main.rs`
  Responsibility: enable the Windows GUI subsystem for non-debug builds only.
- Verify: `dist/index.html`
  Responsibility: confirm Vite emitted relative JS/CSS paths after build.

### Task 1: Add the frontend favicon asset

**Files:**
- Create: `public/favicon.ico`
- Source asset to copy from: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Create the static asset directory if it does not exist**

Run: `ls /opt/cyber-link`
Expected: project root is visible and `public/` may be absent.

Run: `mkdir -p "/opt/cyber-link/public"`
Expected: command succeeds with no output.

- [ ] **Step 2: Copy the existing Tauri icon into the Vite public directory**

Run: `cp "/opt/cyber-link/src-tauri/icons/icon.ico" "/opt/cyber-link/public/favicon.ico"`
Expected: command succeeds with no output.

- [ ] **Step 3: Verify the favicon file now exists**

Run: `ls "/opt/cyber-link/public"`
Expected: output contains `favicon.ico`.

- [ ] **Step 4: Commit the asset addition**

```bash
git add public/favicon.ico
git commit -m "fix: add packaged favicon asset"
```

### Task 2: Emit relative frontend asset paths

**Files:**
- Modify: `vite.config.ts`
- Verify: `dist/index.html`

- [ ] **Step 1: Write the failing packaging expectation down**

Current built output uses root-relative asset URLs and is expected to contain lines like:

```html
<script type="module" crossorigin src="/assets/index-*.js"></script>
<link rel="stylesheet" crossorigin href="/assets/index-*.css">
```

This is the behavior to replace with relative `./assets/...` references.

- [ ] **Step 2: Update `vite.config.ts` to use a relative base path**

Change the returned config object to include `base: './',`:

```ts
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import {defineConfig, loadEnv} from 'vite';

export default defineConfig(({mode}) => {
  const env = loadEnv(mode, '.', '');
  return {
    base: './',
    plugins: [react(), tailwindcss()],
    define: {
      'process.env.GEMINI_API_KEY': JSON.stringify(env.GEMINI_API_KEY),
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, '.'),
      },
    },
    server: {
      // HMR is disabled in AI Studio via DISABLE_HMR env var.
      // Do not modify; file watching is disabled to prevent flickering during agent edits.
      hmr: process.env.DISABLE_HMR !== 'true',
    },
  };
});
```

- [ ] **Step 3: Build the frontend to verify the packaging output changes**

Run: `npm run build`
Expected: Vite build completes successfully and writes `dist/index.html` plus hashed asset files.

- [ ] **Step 4: Verify built asset URLs are relative and the favicon was copied**

Run: `rg -n "assets/|favicon\.ico" "/opt/cyber-link/dist/index.html"`
Expected: asset references begin with `./assets/` and the page may include or successfully coexist with `favicon.ico` in the build output.

Run: `ls "/opt/cyber-link/dist"`
Expected: output contains `favicon.ico`.

- [ ] **Step 5: Commit the Vite packaging change**

```bash
git add vite.config.ts dist/index.html dist/favicon.ico
git commit -m "fix: use desktop-safe frontend asset paths"
```

### Task 3: Remove the Windows release console window

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs` existing Rust test suite remains green

- [ ] **Step 1: Add the Windows GUI subsystem attribute at the top of `main.rs`**

Insert this line before the first `use` statement:

```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
```

Resulting file start:

```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};
```

- [ ] **Step 2: Run the Rust test suite to verify the entry-point change did not break compilation**

Run: `cargo test`
Workdir: `/opt/cyber-link/src-tauri`
Expected: tests pass.

- [ ] **Step 3: Commit the Windows subsystem change**

```bash
git add src-tauri/src/main.rs
git commit -m "fix: hide Windows console for release startup"
```

### Task 4: Final verification of the full fix

**Files:**
- Verify: `public/favicon.ico`
- Verify: `vite.config.ts`
- Verify: `src-tauri/src/main.rs`
- Verify: `dist/index.html`

- [ ] **Step 1: Rebuild the frontend from a clean state**

Run: `npm run clean && npm run build`
Expected: clean succeeds, then Vite rebuilds `dist/` successfully.

- [ ] **Step 2: Re-check the exact desktop packaging expectations**

Run: `rg -n "\./assets/|/assets/|favicon\.ico" "/opt/cyber-link/dist/index.html"`
Expected: matches include `./assets/`; there should be no root-relative `/assets/` match.

Run: `ls "/opt/cyber-link/dist"`
Expected: output includes `favicon.ico`.

- [ ] **Step 3: Build the Tauri app locally for Windows verification**

Run: `cargo tauri build`
Workdir: `/opt/cyber-link/src-tauri`
Expected: Windows bundle/executable is produced in the Tauri target output directory.

- [ ] **Step 4: Launch the built Windows app and verify the original regressions are gone**

Manual check on Windows:

```text
1. Start the installed or bundled app normally.
2. Confirm no black console window appears.
3. Confirm startup no longer shows: Asset favicon.ico not found; fallback to favicon.ico.html
4. Confirm the tray app still starts and behaves normally.
```

Expected: all four checks succeed.

- [ ] **Step 5: Commit the final verified state**

```bash
git add public/favicon.ico vite.config.ts src-tauri/src/main.rs docs/superpowers/specs/2026-04-18-windows-startup-fixes-design.md docs/superpowers/plans/2026-04-18-windows-startup-fixes.md
git commit -m "fix: clean up Windows startup packaging"
```
