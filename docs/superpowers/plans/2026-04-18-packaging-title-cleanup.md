# Packaging Title Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** add standard Tauri npm scripts and replace the template HTML title with the app title.

**Architecture:** This is a configuration-only change. Update the top-level npm scripts for predictable Tauri entrypoints, then align the static HTML metadata with the desktop application identity. No runtime logic changes are required.

**Tech Stack:** npm, Vite, Tauri v1, static HTML

---

### Task 1: Standardize Package Scripts

**Files:**
- Modify: `package.json`
- Test: `package.json`

- [ ] **Step 1: Add the Tauri CLI dependency and scripts**

```json
{
  "scripts": {
    "dev": "vite --port=5173 --host=0.0.0.0",
    "build": "vite build",
    "preview": "vite preview",
    "clean": "rm -rf dist",
    "lint": "tsc --noEmit",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^1.6.0"
  }
}
```

- [ ] **Step 2: Run build to verify existing frontend behavior is unchanged**

Run: `npm run build`
Expected: Vite build completes successfully and writes files under `dist/`.

### Task 2: Align Static HTML Title

**Files:**
- Modify: `index.html`
- Test: `index.html`

- [ ] **Step 1: Replace the template page title**

```html
<title>CyberLink</title>
```

- [ ] **Step 2: Verify the updated title is present in source**

Run: inspect `index.html`
Expected: the document title matches `CyberLink`.

### Task 3: Final Verification

**Files:**
- Test: `package.json`
- Test: `index.html`

- [ ] **Step 1: Run the final build verification**

Run: `npm run build`
Expected: successful build with no new errors.

- [ ] **Step 2: Confirm the intended diff only touches configuration files**

Run: `git diff -- package.json index.html`
Expected: diff shows only npm script/dependency additions and the HTML title update.
