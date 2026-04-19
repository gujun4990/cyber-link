import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('app hides the native window from minimize and close controls', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /import \{ appWindow, LogicalSize \} from '@tauri-apps\/api\/window';/);
  assert.match(appSource, /await appWindow\.hide\(\);/);
  assert.equal(appSource.includes('layoutId="tray-icon"'), false);
  assert.equal(appSource.includes('setIsMinimized'), false);
});

test('showing the main window keeps it at card size', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const sizeFile = JSON.parse(
    readFileSync(new URL('./shared/windowSize.json', import.meta.url), 'utf8'),
  );

  assert.equal(sizeFile.width, 700);
  assert.equal(sizeFile.height, 438);
  assert.equal(appSource.includes("import windowSize from './shared/windowSize.json';"), true);
  assert.equal(appSource.includes('width: windowSize.width,'), true);
  assert.equal(appSource.includes('height: windowSize.height,'), true);
  assert.equal(appSource.includes('await appWindow.setSize(new LogicalSize(windowSize.width, windowSize.height));'), true);
  assert.equal(appSource.includes('await appWindow.show();'), true);
  assert.ok(
    appSource.indexOf('await appWindow.show();') <
      appSource.indexOf("invoke<DeviceState>('initialize_app')"),
  );
  assert.equal(mainSource.includes('include_str!("../../src/shared/windowSize.json")'), true);
  assert.equal(mainSource.includes('let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {'), true);
});

test('restoring an existing main window also reapplies the shared card size', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('SetWindowPos'), true);
  assert.equal(mainSource.includes('main_window_size()'), true);
  assert.equal(mainSource.includes('ShowWindow(hwnd, SW_RESTORE);'), true);
  assert.equal(mainSource.includes('apply_main_window_size_to_hwnd(hwnd)'), true);
});

test('manual startup waits for frontend before showing the window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(
    mainSource.includes('StartupMode::Manual => {}'),
    true,
  );
});
