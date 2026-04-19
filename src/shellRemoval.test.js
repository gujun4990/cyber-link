import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import * as ts from 'typescript';

test('App root renders the card as the only visible surface', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');
  const sf = ts.createSourceFile('App.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);

  const appFn = sf.statements.find(
    (statement) => ts.isFunctionDeclaration(statement) && statement.name?.text === 'App',
  );

  assert.ok(appFn && ts.isFunctionDeclaration(appFn));

  const returnStmt = appFn.body?.statements.find(ts.isReturnStatement);
  const returned = returnStmt?.expression;
  const jsxNode = returned && ts.isParenthesizedExpression(returned) ? returned.expression : returned;
  assert.ok(jsxNode && ts.isJsxFragment(jsxNode));

  const visibleChildren = jsxNode.children.filter((child) => {
    return !ts.isJsxText(child) || child.getText(sf).trim().length > 0;
  });

  assert.equal(visibleChildren.length, 1);
  assert.ok(ts.isJsxElement(visibleChildren[0]) || ts.isJsxSelfClosingElement(visibleChildren[0]));

  const card = visibleChildren[0];
  const opening = ts.isJsxElement(card) ? card.openingElement : card;
  const tagName = opening.tagName.getText(sf);
  assert.equal(tagName, 'motion.div');

  const className = opening.attributes.properties
    .find((attr) => ts.isJsxAttribute(attr) && attr.name.text === 'className');
  assert.ok(className && ts.isJsxAttribute(className));
  assert.match(className.initializer?.getText(sf) ?? '', /fixed inset-0 m-auto/);
  assert.equal(className.initializer?.getText(sf).includes('rounded-xl'), false);
  assert.equal(source.includes('bg-[#020617]/40'), false);
  assert.equal(source.includes('flex items-center justify-center p-4 bg-[#020617]/40'), false);
  assert.equal(source.includes('className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/40 border-b border-white/5 backdrop-blur-xl select-none"\n            onMouseDown'), true);
  assert.equal(source.includes('className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/40 border-b border-white/5 backdrop-blur-xl select-none"\n            data-tauri-drag-region'), false);

  const styleAttr = opening.attributes.properties
    .find((attr) => ts.isJsxAttribute(attr) && attr.name.text === 'style');
  assert.ok(styleAttr && ts.isJsxAttribute(styleAttr));
  assert.match(styleAttr.initializer?.getText(sf) ?? '', /rgba\(10, 20, 60, 1\)/);
  assert.match(styleAttr.initializer?.getText(sf) ?? '', /rgba\(15, 23, 42, 0\.95\)/);
  assert.equal(styleAttr.initializer?.getText(sf).includes('backdropFilter'), false);

  assert.equal(source.includes('flex min-h-screen items-center justify-center p-4 overflow-hidden'), false);
  assert.equal(source.includes('<div className="flex min-h-screen items-center justify-center p-4 overflow-hidden">'), false);
  assert.equal(source.includes('w-64 flex flex-col py-2'), true);
  assert.equal(source.includes('底栏状态条'), true);
  assert.equal(source.includes('currentTime.toLocaleTimeString'), true);
});

test('card background keeps the blue translucent treatment', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('rgba(10, 20, 60, 1)'), true);
  assert.equal(source.includes('rgba(6,182,212,0.1)'), true);
  assert.equal(source.includes("import windowSize from './shared/windowSize.json';"), true);
  assert.equal(source.includes('width: windowSize.width,'), true);
  assert.equal(source.includes('height: windowSize.height,'), true);
  assert.match(source, /background: `\s*linear-gradient\(135deg, rgba\(15, 23, 42, 0\.95\), rgba\(8, 14, 44, 0\.98\)\),\s*rgba\(10, 20, 60, 1\)\s*`/);
});

test('window size load failures are logged instead of silently ignored', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('failed to load main window size'), true);
  assert.equal(mainSource.includes('main window size'), true);
});

test('logs are written to the app log command', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes("invoke('append_log_message', { message })"), true);
  assert.equal(appSource.includes("const consoleLogLevels = ['warn', 'error'] as const;"), true);
  assert.equal(appSource.includes("consoleLogLevels = ['log', 'info', 'warn', 'error']"), false);
});

test('windows binary is built without a console window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('#![cfg_attr(windows, windows_subsystem = "windows")]'), true);
});

test('startup renders both switches off until state is known', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;'), true);
  assert.equal(source.includes('const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;'), true);
  assert.equal(source.includes('const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;'), true);
  assert.equal(source.includes('const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('active={acDisplayOn}'), true);
  assert.equal(source.includes('active={lightDisplayOn}'), true);
  assert.equal(source.includes("subLabel={acDisplayOn ? '核心运行中' : '已关闭'}"), true);
  assert.equal(source.includes("subLabel={lightDisplayOn ? '强光已开启' : '已关闭'}"), true);
});

test('tauri window is configured as a single transparent surface', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  const [mainWindow] = tauriConfig.tauri.windows;

  assert.equal(mainWindow.label, 'main');
  assert.equal(mainWindow.visible, false);
  assert.equal(mainWindow.decorations, false);
  assert.equal(mainWindow.transparent, true);
});

test('windows tray double click reopens the main window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(mainSource, /show_main_window\(app\)/);
  assert.match(mainSource, /SystemTrayEvent::DoubleClick \{ \.\. \} => show_main_window\(app\)/);
});
