import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function normalizeSource(source) {
  return source.replace(/\s+/g, ' ');
}

test('app hides the native window from minimize and close controls', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /await runtime\.hideWindow\(\);/);
  assert.match(appSource, /await runtime\.minimizeWindow\(\);/);
  assert.equal(appSource.includes('layoutId="tray-icon"'), false);
  assert.equal(appSource.includes('setIsMinimized'), false);
});

test('top bar supports drag but ignores double click', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes('data-tauri-drag-region'), false);
  assert.equal(appSource.includes('onDoubleClickCapture'), true);
  assert.equal(appSource.includes('preventDefault()'), true);
  assert.equal(appSource.includes('stopPropagation()'), true);
  assert.equal(appSource.includes('await runtime.startDragging();'), true);
});

test('top bar supports drag with no native drag region', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes('className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/20 border-b border-white/10 backdrop-blur-sm select-none"\n        onMouseDown'), true);
  assert.equal(appSource.includes('onDoubleClickCapture'), true);
  assert.equal(appSource.includes('preventDefault()'), true);
  assert.equal(appSource.includes('stopPropagation()'), true);
  assert.equal(appSource.includes('await runtime.startDragging();'), true);
});

test('showing the main window keeps it at card size', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const sizeFile = JSON.parse(
    readFileSync(new URL('./shared/windowSize.json', import.meta.url), 'utf8'),
  );

  assert.equal(sizeFile.width, 650);
  assert.equal(sizeFile.height, 500);
  assert.equal(appSource.includes("import windowSize from './shared/windowSize.json';"), true);
  assert.equal(appSource.includes('width: windowSize.width,'), true);
  assert.equal(appSource.includes('height: windowSize.height,'), true);
  assert.equal(appSource.includes('await runtime.setWindowSize(windowSize.width, windowSize.height);'), true);
  assert.equal(appSource.includes('await runtime.showWindow();'), true);
  assert.ok(
    appSource.indexOf('await runtime.showWindow();') <
      appSource.indexOf('runtime.initializeApp(),'),
  );
  assert.equal(appSource.includes('width: windowSize.width,'), true);
  assert.equal(appSource.includes('height: windowSize.height,'), true);
  assert.equal(mainSource.includes('include_str!("../../src/shared/windowSize.json")'), true);
  assert.equal(mainSource.includes('let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {'), true);
});

test('tauri window is fixed size and not resizable', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  const [mainWindow] = tauriConfig.tauri.windows;

  assert.equal(mainWindow.resizable, false);
  assert.equal(mainWindow.maximizable, false);
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

test('autostart keeps the window hidden before initialization completes', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes('await runtime.isAutostartMode();'), true);
  assert.equal(appSource.includes('if (!autostartMode) {'), true);
  assert.equal(appSource.includes('await runtime.showWindow();'), true);
});

test('state refresh listener cleanup handles late async registration', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes('let disposed = false;'), true);
  assert.equal(appSource.includes('if (disposed) return;'), true);
  assert.equal(appSource.includes('disposed = true;'), true);
});

test('window minimize and hide only pause after the runtime call succeeds', () => {
  const appSource = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /await runtime\.hideWindow\(\);[\s\S]*setUiPaused\(true\);/);
  assert.match(appSource, /await runtime\.minimizeWindow\(\);[\s\S]*setUiPaused\(true\);/);
});

test('pause state is driven by visibility only', () => {
  const appSource = normalizeSource(readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8'));

  assert.match(appSource, /document\.addEventListener\('visibilitychange', syncUiPaused\)/);
  assert.equal(appSource.includes("window.addEventListener('blur', syncUiPaused)"), false);
  assert.equal(appSource.includes("window.addEventListener('focus', syncUiPaused)"), false);
});
