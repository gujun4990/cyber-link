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

test('app source carries initialization errors through to the UI', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /initError\?: string/);
  assert.match(appSource, /const msg = error instanceof Error \? error\.message : String\(error\);/);
  assert.match(appSource, /connected: false, initError: msg/);
  assert.match(appSource, /初始化失败:/);
  assert.match(appSource, /服务器连接失败/);
});

test('windows entrypoint restores the existing main window on relaunch', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(mainSource, /try_restore_existing_main_window/);
  assert.match(mainSource, /static INSTANCE_MUTEX: OnceLock<isize> = OnceLock::new\(\);/);
  assert.match(mainSource, /CreateMutexW/);
  assert.match(mainSource, /SetLastError\(0\);/);
  assert.match(mainSource, /ERROR_ALREADY_EXISTS/);
  assert.match(mainSource, /FindWindowW/);
  assert.match(mainSource, /SW_RESTORE/);
  assert.match(mainSource, /SetForegroundWindow/);
  assert.match(mainSource, /if try_restore_existing_main_window\(\) \{[\s\S]*return;[\s\S]*\}/);
});

test('windows entrypoint does not wire a system tray icon', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('.system_tray(build_tray())'), false);
  assert.equal(mainSource.includes('.on_system_tray_event(handle_tray_event)'), false);
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
