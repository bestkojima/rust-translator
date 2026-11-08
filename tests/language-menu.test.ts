import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { escapeLanguageMenu, languageChanged, moveLanguageOption, toggleLanguageMenu } from '../src/language-menu.ts';

test('the same menu toggles closed and switching menus leaves exactly one open', () => {
  assert.equal(toggleLanguageMenu(null, 'source'), 'source');
  assert.equal(toggleLanguageMenu('source', 'source'), null);
  assert.equal(toggleLanguageMenu('source', 'target'), 'target');
  assert.equal(toggleLanguageMenu('target', 'source'), 'source');
});

test('Escape dismisses the menu before the floating window', () => {
  assert.deepEqual(escapeLanguageMenu('source'), { menu: null, closeWindow: false });
  assert.deepEqual(escapeLanguageMenu('target'), { menu: null, closeWindow: false });
  assert.deepEqual(escapeLanguageMenu(null), { menu: null, closeWindow: true });
});

test('selecting the current language does not trigger another translation', () => {
  assert.equal(languageChanged('zh', 'zh'), false);
  assert.equal(languageChanged('zh', 'en'), true);
});

test('keyboard navigation wraps and supports Home and End', () => {
  assert.equal(moveLanguageOption(2, 3, 'ArrowDown'), 0);
  assert.equal(moveLanguageOption(0, 3, 'ArrowUp'), 2);
  assert.equal(moveLanguageOption(1, 3, 'Home'), 0);
  assert.equal(moveLanguageOption(1, 3, 'End'), 2);
  assert.equal(moveLanguageOption(0, 0, 'ArrowDown'), 0);
});
