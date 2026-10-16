import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const source = readFileSync(new URL('../web/browser.js', import.meta.url), 'utf8')
  .replace("import { touchControls } from './touch.js';", '')
  .split('// Generated bindings cache the audio constructor')[0];

function browser({ mode = 'browser', standalone = false, hidden = false } = {}) {
  function target(properties = {}) {
    const listeners = new Map();
    return {
      ...properties,
      addEventListener(name, callback) {
        if (!listeners.has(name)) listeners.set(name, []);
        listeners.get(name).push(callback);
      },
      dispatch(name, event = {}) {
        for (const callback of listeners.get(name) || []) callback(event);
      },
    };
  }
  const elements = Object.fromEntries(['player', 'game', 'overlay', 'status', 'play', 'fullscreen']
    .map(id => [id, target({ hidden: false, focus() {}, contains: () => true })]));
  const document = target({
    hidden,
    hasFocus: () => true,
    querySelector: selector => elements[selector.slice(1)],
  });
  let resumes = 0;
  class AudioContext {
    state = 'suspended';
    resume() {
      resumes++;
      // Autoplay denial can leave resume pending until a user gesture.
      return new Promise(() => {});
    }
    suspend() {
      this.state = 'suspended';
      return Promise.resolve();
    }
  }
  const window = target({ AudioContext });
  const touch = { enabled: true, active: false, setActive(active) { this.active = active; } };
  runInNewContext(source, {
    window, document, navigator: { standalone }, console,
    matchMedia: query => ({ matches: query.includes(`display-mode: ${mode}`) }),
    touchControls: () => touch,
  });
  const audio = new window.AudioContext();
  window.tempestBrowser.ready();
  return { window, document, elements, touch, audio, resumes: () => resumes };
}

test('normal tabs keep Play, installed display modes and iOS standalone skip it', () => {
  const tab = browser();
  assert.equal(tab.window.tempestBrowser.active(), false);
  assert.equal(tab.elements.overlay.hidden, false);
  tab.elements.play.dispatch('click');
  assert.equal(tab.window.tempestBrowser.active(), true);
  for (const options of [
    { mode: 'fullscreen' }, { mode: 'standalone' }, { mode: 'minimal-ui' }, { standalone: true },
  ]) {
    const app = browser(options);
    assert.equal(app.elements.overlay.hidden, true);
    assert.equal(app.window.tempestBrowser.active(), true);
    assert.equal(app.touch.active, true);
  }
});

test('normal player touches and keys retry blocked audio without consuming input', () => {
  const app = browser({ mode: 'standalone' });
  assert.equal(app.resumes(), 1);
  app.elements.player.dispatch('pointerdown');
  assert.equal(app.resumes(), 2);
  app.elements.player.dispatch('pointerup');
  assert.equal(app.resumes(), 3);
  app.document.dispatch('keydown', { code: 'ArrowLeft' });
  assert.equal(app.resumes(), 4);
  app.audio.state = 'running';
  app.elements.player.dispatch('pointerdown');
  assert.equal(app.resumes(), 4);
  app.audio.state = 'closed';
  app.document.dispatch('keydown', { code: 'ArrowLeft' });
  assert.equal(app.resumes(), 4);
});

test('installed apps still suspend and require Resume on return', () => {
  const app = browser({ standalone: true });
  app.document.hidden = true;
  app.document.dispatch('visibilitychange');
  app.document.hidden = false;
  app.document.dispatch('visibilitychange');
  assert.equal(app.elements.overlay.hidden, false);
  assert.equal(app.elements.play.textContent, 'Resume');
  assert.equal(app.touch.active, false);
  assert.equal(app.window.tempestBrowser.active(), false);
  assert.equal(app.window.tempestBrowser.active(), false);
  app.elements.play.dispatch('click');
  assert.equal(app.window.tempestBrowser.active(), true);
});

test('installed apps do not engage while loading in the background', () => {
  const app = browser({ mode: 'standalone', hidden: true });
  assert.equal(app.elements.overlay.hidden, false);
  assert.equal(app.resumes(), 0);
  assert.equal(app.touch.active, false);
});
