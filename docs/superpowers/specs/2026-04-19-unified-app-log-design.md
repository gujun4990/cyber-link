# Unified App Log Design

**Goal:** Write warning and error logs into `C:\Users\<you>\AppData\Local\cyber-link\app.log`.

**Architecture:** Keep the current app structure, but replace scattered stderr/console logging with a single app-log sink. Rust backend messages and React frontend messages will both append to the same file under the app-local directory, while still preserving stderr/console output as a development fallback.

**Tech Stack:** Rust, React, Tauri 1, TypeScript

---

### Logging Scope

Only warning and error logs should be routed into `app.log`.

The app should continue to print `log/info` to the normal console/devtools path only.

The existing app-local path for configuration will also be reused as the base for the log file, and the app-local directory will be created on startup if it does not exist.

**Looks right so far?**

### Non-Goals

- No external logging service
- No new UI for logs
- No change to app behavior beyond where logs are written
