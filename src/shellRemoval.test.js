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
  assert.ok(jsxNode && ts.isJsxElement(jsxNode));

  const opening = jsxNode.openingElement;
  assert.equal(opening.tagName.getText(sf), 'motion.div');

  const className = opening.attributes.properties
    .find((attr) => ts.isJsxAttribute(attr) && attr.name.text === 'className');
  assert.ok(className && ts.isJsxAttribute(className));
  assert.match(className.initializer?.getText(sf) ?? '', /fixed inset-0 m-auto/);
  assert.match(className.initializer?.getText(sf) ?? '', /overflow-hidden flex flex-col/);
  assert.equal(className.initializer?.getText(sf).includes('rounded-xl'), false);
  assert.equal(source.includes('bg-[#020617]/40'), false);
  assert.equal(source.includes('flex items-center justify-center p-4 bg-[#020617]/40'), false);
  assert.equal(source.includes('className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/20 border-b border-white/10 backdrop-blur-sm select-none"\n        onMouseDown'), true);
  assert.equal(source.includes('className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/20 border-b border-white/10 backdrop-blur-sm select-none"\n        data-tauri-drag-region'), false);

  const styleAttr = opening.attributes.properties
    .find((attr) => ts.isJsxAttribute(attr) && attr.name.text === 'style');
  assert.ok(styleAttr && ts.isJsxAttribute(styleAttr));
  assert.match(styleAttr.initializer?.getText(sf) ?? '', /appShellStyle/);

  assert.equal(source.includes('flex min-h-screen items-center justify-center p-4 overflow-hidden'), false);
  assert.equal(source.includes('<div className="flex min-h-screen items-center justify-center p-4 overflow-hidden">'), false);
  assert.equal(source.includes('w-[248px] flex flex-col py-2'), true);
  assert.equal(source.includes('当前时间'), false);
  assert.equal(source.includes('currentTime.toLocaleTimeString'), true);
});

test('card background keeps the blue translucent treatment', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('rgba(28, 48, 118, 0.97)'), true);
  assert.equal(source.includes('rgba(15, 25, 70, 1)'), true);
  assert.equal(source.includes("import windowSize from './shared/windowSize.json';"), true);
  assert.equal(source.includes('width: windowSize.width,'), true);
  assert.equal(source.includes('height: windowSize.height,'), true);
  assert.match(source, /background: `\s*radial-gradient\(circle at 20% 20%, rgba\(70, 110, 255, 0\.5\), transparent 55%\),\s*linear-gradient\(135deg, rgba\(28, 48, 118, 0\.97\), rgba\(15, 25, 70, 1\)\)\s*`/);
});

test('window size load failures are logged instead of silently ignored', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('failed to load main window size'), true);
  assert.equal(mainSource.includes('main window size'), true);
});

test('logs are written to the app log command', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(appSource.includes('runtime.appendLogMessage(message)'), true);
  assert.equal(appSource.includes("const consoleLogLevels = ['warn', 'error'] as const;"), true);
  assert.equal(appSource.includes("consoleLogLevels = ['log', 'info', 'warn', 'error']"), false);
  assert.equal(appSource.includes('new Date().toISOString()'), true);
  assert.equal(appSource.includes('reportError('), true);
});

test('windows binary is built without a console window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('#![cfg_attr(windows, windows_subsystem = "windows")]'), true);
});

test('startup renders lighting cards with the configured count', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('ambientLightAvailable: device.ambientLightAvailable,'), true);
  assert.equal(source.includes('ambientLightOn: device.ambientLightOn,'), true);
  assert.equal(source.includes('buildLightingCards({'), true);
  assert.equal(source.includes('const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;'), true);
  assert.equal(source.includes('const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;'), true);
  assert.equal(source.includes('const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('const [syncingAction, setSyncingAction] = useState(false);'), true);
  assert.equal(source.includes('hasLoadedState && lightingCards.length > 0'), true);
  assert.equal(source.includes('renderLightingToggle(lightingCards[0])'), true);
  assert.equal(source.includes('lightingCards.length === 2'), true);
  assert.equal(source.includes('lightingCards.length === 3'), true);
  assert.equal(source.includes('lightingCount'), false);
  assert.equal(source.includes('照明控制'), true);
  assert.equal(source.includes('grid grid-cols-2 gap-3'), true);
  assert.equal(source.includes('col-span-2'), false);
  assert.equal(source.includes('未配置灯光'), false);
});

test('compact lighting cards do not truncate their labels', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(source, /isCompact\s*\?\s*'text-\[10px\] tracking-\[0\.04em\] whitespace-nowrap'/);
  assert.match(source, /isCompact\s*\?\s*'text-\[8px\] whitespace-nowrap'\s*:\s*'text-\[10px\] truncate'/);
  assert.equal(source.includes('className={`font-black antialiased transition-colors duration-300 truncate ${'), false);
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
