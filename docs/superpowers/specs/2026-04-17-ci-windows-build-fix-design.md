# CI Windows Build Fix Design

## Goal

Restore the GitHub `Release` workflow by fixing the current Windows build failures without modifying any CSS or UI component structure.

## Scope

In scope:
- Fix the Windows-only Rust compile failures in `src-tauri/src/main.rs`.
- Replace the broken Windows icon resource consumed by Tauri build.
- Add tests first and use a TDD loop for each fix.
- Verify the targeted build path that currently fails in CI.

Out of scope:
- Any CSS changes.
- Any React or UI component structure changes.
- Workflow cleanup unrelated to the current failure.
- Product behavior changes beyond preserving the intended existing shutdown-hook behavior.

## Problem Summary

The `Release` workflow fails in the `Build and release Windows installer` step because `tauri build --bundles nsis` fails on Windows for two independent reasons:

1. `src-tauri/icons/icon.ico` is invalid and cannot be decoded by Tauri during context generation.
2. The Windows shutdown-hook code in `src-tauri/src/main.rs` uses Windows handle/result types in a way that does not match the current `windows-sys` APIs used by CI.

These are repository-content failures, not workflow-syntax failures.

## Approach Options Considered

### Option 1: Minimal CI repair

Fix the icon resource and update the Windows-only Rust code to the correct `windows-sys` type usage while keeping behavior the same.

Pros:
- Smallest safe change.
- Matches the requested scope.
- Avoids frontend and workflow churn.

Cons:
- Leaves unrelated workflow warnings untouched.

### Option 2: Temporarily remove the shutdown hook

Disable the Windows hook so the app builds without that code path.

Pros:
- Fast.

Cons:
- Risks behavior regression during Windows shutdown.
- Changes product behavior unnecessarily.

### Option 3: Broader release cleanup

Repair the CI failure and also restructure workflow/build configuration.

Pros:
- Cleaner long term release pipeline.

Cons:
- Larger change surface than needed.
- Harder to verify as a focused bug fix.

## Chosen Design

Use Option 1.

The fix will preserve the current frontend and visual structure entirely. Only Windows build assets and Windows-specific Rust code will change.

## Implementation Design

### 1. TDD sequence

For each bug, start with a failing test or failing verification artifact before production changes:

- Add a narrow Rust test for any extracted or directly testable Windows-hook helper behavior.
- Add a resource-validity test that proves the icon file is present and structurally valid enough for build consumption.
- Run the relevant tests first and confirm failure for the expected reason.
- Implement the minimal production change.
- Re-run the focused tests and then broader verification.

Because some failures are compile-time/platform-specific, the TDD loop may include a failing targeted build/check command in addition to unit tests. The key requirement is still red first, then green.

### 2. Windows Rust code repair

Update `src-tauri/src/main.rs` so the shutdown-hook implementation matches the `windows-sys` API shapes used in CI:

- Stop using tuple-struct field access on raw pointer aliases where the current type is not a tuple struct.
- Pass the correct `HWND` type to `SetWindowLongPtrW`.
- Return `LRESULT` using the correct type form for the current crate version.
- Keep the existing shutdown-trigger and previous-window-proc chaining behavior unchanged.

If needed for testability, extract only the smallest non-UI helper needed to validate key conversions or store behavior. Avoid broader refactors.

### 3. Icon resource repair

Replace `src-tauri/icons/icon.ico` with a valid icon file that Tauri can decode on Windows.

Constraints:
- Do not change the UI structure.
- Do not introduce new visual features.
- Keep the asset path stable so no config changes are required unless validation shows one is necessary.

The icon may be regenerated from the existing PNG source if that is the safest minimal path, provided the resulting file is valid.

## Testing Strategy

### Focused red/green tests

- Rust tests covering the directly testable logic touched by the Windows hook fix.
- An asset validity test for the icon resource.

### Verification commands

- Run the focused Rust test target(s).
- Run a targeted Rust check/build path for the Tauri Windows code where feasible.
- Run the smallest build command that demonstrates the original failure no longer reproduces.

If cross-compiling Windows from the current environment is not feasible, document that limitation and rely on the strongest local verification available plus the exact CI failure mapping.

## Risks And Mitigations

- Risk: The icon test may prove local validity but still differ from Tauri's decoder behavior.
  Mitigation: Validate the actual `.ico` file shape and use the same file path consumed by Tauri.

- Risk: Windows-only APIs are hard to verify on Linux.
  Mitigation: Keep the change minimal, isolate touched logic, and use targeted compile/test coverage where available.

- Risk: Broader refactoring could create regressions.
  Mitigation: Do not refactor unrelated code.

## Success Criteria

- No CSS files are changed.
- No UI component structure is changed.
- `src-tauri/src/main.rs` no longer contains the current Windows type errors seen in CI.
- `src-tauri/icons/icon.ico` is valid and no longer triggers the Tauri icon decode failure.
- Local focused tests and verification pass.
- The repository is ready for the GitHub `Release` workflow to rerun without the current failure mode.
