import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('tauri config embeds the WebView2 bootstrapper for Windows installs', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  assert.equal(
    tauriConfig.tauri.bundle.windows.webviewInstallMode.type,
    'embedBootstrapper',
  );
});

test('tauri config installs NSIS releases for the current user', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  assert.equal(tauriConfig.tauri.bundle.windows.nsis.installMode, 'currentUser');
});

test('app source hides the native window from the title buttons', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /const hideWindow = async \(\) =>/);
  assert.match(appSource, /const minimizeWindow = async \(\) =>/);
  assert.match(appSource, /await appWindow\.hide\(\);/);
  assert.match(appSource, /OFFLINE_MODE/);
  assert.match(appSource, /ACTION_FAILED/);
  assert.match(appSource, /REFRESH_FAILED/);
});

test('windows entrypoint restores the existing main window on relaunch', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(mainSource, /try_restore_existing_main_window/);
  assert.match(mainSource, /static INSTANCE_MUTEX: OnceLock<usize> = OnceLock::new\(\);/);
  assert.match(mainSource, /CreateMutexW/);
  assert.match(mainSource, /SetLastError\(0\);/);
  assert.match(mainSource, /ERROR_ALREADY_EXISTS/);
  assert.match(mainSource, /FindWindowW/);
  assert.match(mainSource, /SW_RESTORE/);
  assert.match(mainSource, /SetForegroundWindow/);
  assert.match(mainSource, /if try_restore_existing_main_window\(\) \{[\s\S]*return;[\s\S]*\}/);
});

test('windows entrypoint wires a system tray menu for open and exit', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(mainSource, /\.system_tray\(build_tray\(\)\)/);
  assert.match(mainSource, /\.on_system_tray_event\(handle_tray_event\)/);
  assert.match(mainSource, /TRAY_OPEN_ID/);
  assert.match(mainSource, /TRAY_EXIT_ID/);
  assert.match(mainSource, /WindowEvent::CloseRequested/);
  assert.match(mainSource, /event\.window\(\)\.hide\(\)/);
  assert.match(mainSource, /app\.exit\(0\)/);
});

test('release workflow builds frontend before tauri packaging', () => {
  const workflow = readFileSync(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  );

  const installDeps = workflow.indexOf('name: Install frontend dependencies');
  const buildFrontend = workflow.indexOf('name: Build frontend');
  const buildTauri = workflow.indexOf('name: Build Tauri app');

  assert.notEqual(installDeps, -1);
  assert.notEqual(buildFrontend, -1);
  assert.notEqual(buildTauri, -1);
  assert.ok(installDeps < buildFrontend);
  assert.ok(buildFrontend < buildTauri);
  assert.match(workflow, /run: npm run tauri:build -- --bundles nsis/);
});
