// Bits match browser::touch_keys. Each pointer owns its action independently.
export function touchControls(player, canvas) {
  const root = player.querySelector('#touch-controls');
  const pad = player.querySelector('#steering-pad');
  const buttons = [...root.querySelectorAll('[data-action]')];
  const coarse = matchMedia('(any-pointer: coarse)');
  const pointers = new Map();
  let enabled = coarse.matches || navigator.maxTouchPoints > 0;
  let active = false;
  let context = '';
  let pending = 0;
  let drag = null;
  let feedbackHeld = 0;

  function vibrate() {
    if (enabled && active) {
      try { navigator.vibrate?.(4); } catch { /* Unavailable in some browser contexts. */ }
    }
  }

  function paint(feedback = true) {
    const held = [...pointers.values()].reduce((mask, action) => mask | action, 0);
    const changed = held ^ feedbackHeld;
    if (feedback && changed) vibrate();
    feedbackHeld = held;
    for (const button of buttons) {
      button.classList.toggle('held', !!(held & Number(button.dataset.action)));
    }
    pad.classList.toggle('held', drag !== null);
    if (!drag) pad.removeAttribute('style');
  }

  function reset() {
    const ids = [...pointers.keys()];
    pointers.clear();
    drag = null;
    pending = 0;
    for (const id of ids) {
      if (root.hasPointerCapture(id)) root.releasePointerCapture(id);
    }
    paint(false);
  }

  function display() {
    root.hidden = !enabled;
    player.classList.toggle('touch-visible', enabled);
  }

  coarse.addEventListener('change', () => {
    enabled = coarse.matches || navigator.maxTouchPoints > 0;
    reset();
    display();
  });
  player.addEventListener('pointerdown', event => {
    if (event.pointerType === 'touch' && !enabled) {
      enabled = true;
      display();
    }
    if (!active || event.button !== 0) return;
    if (context === 'gameplay' && [player, root].includes(event.target)) pending |= 128;
  });

  function actionAt(x, y) {
    const button = document.elementFromPoint(x, y)?.closest('[data-action]');
    return button && root.contains(button) && !button.hidden ? Number(button.dataset.action) : 0;
  }

  root.addEventListener('pointerdown', event => {
    if (!active || !enabled || event.button !== 0) return;
    const steering = pad.contains(event.target);
    if (steering && drag) return;
    const action = actionAt(event.clientX, event.clientY);
    if (!action && !steering) return;
    event.preventDefault();
    canvas.focus({ preventScroll: true });
    pointers.set(event.pointerId, action);
    pending |= action;
    if (steering) {
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
      steer(event);
    }
    root.setPointerCapture(event.pointerId);
    paint();
  });
  function steer(event) {
    // Follow long slides so reversing never needs a trip across the whole pad.
    const x = Math.max(-32, Math.min(32, event.clientX - drag.x));
    const y = Math.max(-32, Math.min(32, event.clientY - drag.y));
    drag.x = event.clientX - x;
    drag.y = event.clientY - y;
    const action = (x < -10 ? 1 : x > 10 ? 2 : 0) | (y < -10 ? 4 : y > 10 ? 8 : 0);
    pointers.set(event.pointerId, action);
    pending = (pending & ~15) | action;
    const bounds = pad.getBoundingClientRect();
    pad.style.setProperty('--origin-x', `${drag.x - bounds.left}px`);
    pad.style.setProperty('--origin-y', `${drag.y - bounds.top}px`);
    pad.style.setProperty('--drag-x', `${x}px`);
    pad.style.setProperty('--drag-y', `${y}px`);
  }
  root.addEventListener('pointermove', event => {
    if (!pointers.has(event.pointerId)) return;
    event.preventDefault();
    if (event.pointerId === drag?.id) {
      steer(event);
    } else {
      const action = actionAt(event.clientX, event.clientY);
      if (action & 112) {
        pointers.set(event.pointerId, action);
        pending |= action;
      }
    }
    paint();
  });
  function release(event) {
    pointers.delete(event.pointerId);
    if (event.pointerId === drag?.id) drag = null;
    paint();
  }
  root.addEventListener('pointerup', release);
  root.addEventListener('pointercancel', () => reset());
  root.addEventListener('lostpointercapture', release);
  root.addEventListener('contextmenu', event => event.preventDefault());
  window.addEventListener('resize', reset);
  screen.orientation?.addEventListener('change', reset);
  display();

  return {
    reset,
    get enabled() { return enabled; },
    setActive(value) {
      active = value;
      if (!active) reset();
      display();
    },
    read(nextContext) {
      if (nextContext !== context) {
        if (context === 'pause' || nextContext === 'pause') vibrate();
        reset();
        context = nextContext;
        const menu = context !== 'gameplay';
        player.querySelector('#touch-fire').textContent = menu ? 'OK' : 'Fire';
        player.querySelector('#touch-jump').textContent = menu ? 'Back' : 'Jump';
        player.querySelector('#touch-zapper').hidden = menu;
        player.querySelector('#touch-settings').hidden = !['welcome', 'configuration'].includes(context);
      }
      if (!active || !enabled) return 0;
      const mask = [...pointers.values()].reduce((held, action) => held | action, pending);
      pending = 0;
      return mask;
    },
  };
}
