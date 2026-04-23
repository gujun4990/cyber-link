import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function loadSource(relativeUrl) {
  return readFileSync(new URL(relativeUrl, import.meta.url), 'utf8');
}

function normalizeSource(source) {
  return source.replace(/\s+/g, ' ').trim();
}

function assertIncludesAll(source, snippets) {
  for (const snippet of snippets) {
    assert.ok(
      source.includes(snippet),
      `expected source to include ${JSON.stringify(snippet)}`,
    );
  }
}

test('app uses reduced-motion gating for the main ring', () => {
  const appSource = normalizeSource(loadSource('./App.tsx'));

  assertIncludesAll(appSource, [
    'useReducedMotion',
    'prefersReducedMotion',
    'duration: 34',
    'repeat: Infinity',
  ]);
});

test('app shell keeps blur light', () => {
  const appSource = loadSource('./App.tsx');

  assert.match(appSource, /backdropFilter:\s*'blur\(2px\) saturate\(108%\)'/);
});

test('css animations have reduced-motion fallback', () => {
  const cssSource = normalizeSource(loadSource('./index.css'));

  assertIncludesAll(cssSource, [
    'animation: iridescent-flow 16s ease infinite',
    'animation: border-flow 12s linear infinite',
    'prefers-reduced-motion: reduce',
    'animation: none',
  ]);
});

test('temperature core keeps glow lighter and preserves compositor hint', () => {
  const appSource = normalizeSource(loadSource('./App.tsx'));

  assertIncludesAll(appSource, ['blur-[10px]', 'blur-[4px]']);
  assert.match(appSource, /willChange:\s*'transform, opacity'/);
});

test('repeated UI shadows are kept modest', () => {
  const appSource = normalizeSource(loadSource('./App.tsx'));

  assertIncludesAll(appSource, [
    'shadow-[0_0_8px_rgba(6,182,212,0.28)]',
    'shadow-[0_0_6px_rgba(6,182,212,0.22)]',
  ]);
});

test('hover and status bar effects stay subdued', () => {
  const appSource = normalizeSource(loadSource('./App.tsx'));

  assertIncludesAll(appSource, [
    'hover:border-cyan-300/25',
    'hover:bg-cyan-400/6',
    'via-cyan-400/35',
    'shadow-[0_0_3px_rgba(103,232,249,0.24)]',
  ]);
});
