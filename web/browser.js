const player = document.querySelector('#player');
const canvas = document.querySelector('#game');
const overlay = document.querySelector('#overlay');
const status = document.querySelector('#status');
const play = document.querySelector('#play');
const fullscreen = document.querySelector('#fullscreen');
const controls = '\n\nMove: arrows / A D · Fire: Space / Z\nJump: X · Superzapper: C\nPause / configuration: P · FPS: F3 · Fullscreen: F\nConfirm: Enter · Back: Esc\n\nControllers supported. Press a controller button to connect.';
const audioContexts = new Set();
let loaded = false;
let engaged = false;
let started = false;
let failed = false;
let pendingSuspension = false;
let typingInitials = false;

// CPAL owns the audio context. Retain it so a user gesture can resume it.
const AudioContext = window.AudioContext || window.webkitAudioContext;
if (AudioContext) {
  window.AudioContext = new Proxy(AudioContext, {
    construct(target, args) {
      const context = Reflect.construct(target, args);
      audioContexts.add(context);
      return context;
    },
  });
}

function fail(error) {
  failed = true;
  engaged = false;
  overlay.hidden = false;
  status.textContent = `The game could not run.\n${error?.message || String(error)}\nTry a browser with WebGL2 and hardware acceleration enabled.`;
  play.disabled = true;
  console.error(error);
}

window.tempestBrowser = {
  active: () => {
    // Observe hide/resume even when both happened between animation frames.
    if (pendingSuspension) {
      pendingSuspension = false;
      return false;
    }
    return engaged && !document.hidden && document.hasFocus();
  },
  ready: () => {
    loaded = true;
    status.textContent = `Click Play to enable sound and focus the game.${controls}`;
    play.disabled = false;
    fullscreen.disabled = false;
  },
  typingInitials: active => {
    typingInitials = active;
  },
};

function engage() {
  if (!loaded || failed || document.hidden) return;
  for (const context of audioContexts) {
    context.resume().catch(error => {
      console.warn('Could not resume audio:', error);
    });
  }
  engaged = true;
  started = true;
  overlay.hidden = true;
  canvas.focus();
}

function suspend() {
  if (!engaged || failed) return;
  engaged = false;
  pendingSuspension = true;
  overlay.hidden = false;
  status.textContent = `Game suspended. Click Resume to continue.${controls}`;
  play.textContent = 'Resume';
  for (const context of audioContexts) context.suspend().catch(console.warn);
}

play.addEventListener('click', engage);
window.addEventListener('blur', suspend);
document.addEventListener('visibilitychange', () => {
  if (document.hidden) suspend();
});
player.addEventListener('focusout', event => {
  if (!player.contains(event.relatedTarget)) suspend();
});
canvas.addEventListener('pointerdown', () => {
  if (started) engage();
});
document.addEventListener('keydown', event => {
  if (!overlay.hidden && loaded && ['Enter', 'KeyP'].includes(event.code)) {
    event.preventDefault();
    event.stopImmediatePropagation();
    engage();
  }
}, true);
canvas.addEventListener('keydown', event => {
  if (event.code === 'KeyF' && started && !typingInitials
      && !event.repeat && !event.ctrlKey && !event.altKey && !event.metaKey) {
    event.preventDefault();
    event.stopImmediatePropagation();
    toggleFullscreen();
    return;
  }
  if (['F3', 'Space', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.code)) {
    event.preventDefault();
  }
});
async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await player.requestFullscreen();
    if (started) engage();
  } catch (error) {
    suspend();
    status.textContent = `Fullscreen is unavailable: ${error.message}`;
    console.warn(error);
  }
}
fullscreen.addEventListener('click', toggleFullscreen);
document.addEventListener('fullscreenchange', () => {
  fullscreen.textContent = document.fullscreenElement ? 'Exit fullscreen' : 'Fullscreen';
});
window.addEventListener('error', event => fail(event.error || event.message));
window.addEventListener('unhandledrejection', event => fail(event.reason));

// Generated bindings cache the audio constructor, so import them after wrapping it.
import('./tempest2032.js').then(({ default: init }) => init()).catch(fail);
