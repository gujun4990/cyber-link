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
  assert.equal(source.includes('bg-[#020617]/40'), false);
  assert.equal(source.includes('flex items-center justify-center p-4 bg-[#020617]/40'), false);

  const styleAttr = opening.attributes.properties
    .find((attr) => ts.isJsxAttribute(attr) && attr.name.text === 'style');
  assert.ok(styleAttr && ts.isJsxAttribute(styleAttr));
  assert.match(styleAttr.initializer?.getText(sf) ?? '', /backdropFilter: 'blur\(40px\)'/);
  assert.match(styleAttr.initializer?.getText(sf) ?? '', /rgba\(10, 20, 60, 1\)/);

  assert.equal(source.includes('flex min-h-screen items-center justify-center p-4 overflow-hidden'), false);
  assert.equal(source.includes('<div className="flex min-h-screen items-center justify-center p-4 overflow-hidden">'), false);
  assert.equal(source.includes('w-64 flex flex-col py-2'), true);
  assert.equal(source.includes('底栏状态条'), true);
  assert.equal(source.includes('currentTime.toLocaleTimeString'), true);
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
