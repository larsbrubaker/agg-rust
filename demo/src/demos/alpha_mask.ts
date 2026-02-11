import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Mask',
    'Lion with elliptical alpha mask — matching C++ alpha_mask.cpp.',
  );

  const W = 512, H = 400;

  // Matches C++ alpha_mask.cpp globals.
  let angle = 0.0;
  let scale = 1.0;
  let skewX = 0;
  let skewY = 0;
  let dragging = false;
  let syncingSidebar = false;

  function draw() {
    renderToCanvas({
      demoName: 'alpha_mask',
      canvas, width: W, height: H,
      params: [angle, scale, skewX, skewY],
      timeDisplay: timeEl,
    });
  }

  function setFromTransformPoint(x: number, y: number) {
    const dx = x - W / 2;
    const dy = y - H / 2;
    angle = Math.atan2(dy, dx);
    scale = Math.max(Math.hypot(dx, dy) / 100.0, 0.01);
  }

  function pointerToAgg(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTop = (e.clientY - rect.top) * sy;
    return { x, y: H - yTop };
  }

  const angleSlider = addSlider(sidebar, 'Angle (rad)', -3.1416, 3.1416, angle, 0.01, v => {
    angle = v;
    if (!syncingSidebar) draw();
  });
  const scaleSlider = addSlider(sidebar, 'Scale', 0.01, 5.0, scale, 0.01, v => {
    scale = v;
    if (!syncingSidebar) draw();
  });
  const skewXSlider = addSlider(sidebar, 'Skew X', 0, W, skewX, 1, v => {
    skewX = v;
    if (!syncingSidebar) draw();
  });
  const skewYSlider = addSlider(sidebar, 'Skew Y', 0, H, skewY, 1, v => {
    skewY = v;
    if (!syncingSidebar) draw();
  });

  function syncSidebar() {
    syncingSidebar = true;
    angleSlider.value = String(angle);
    angleSlider.dispatchEvent(new Event('input'));
    scaleSlider.value = String(scale);
    scaleSlider.dispatchEvent(new Event('input'));
    skewXSlider.value = String(skewX);
    skewXSlider.dispatchEvent(new Event('input'));
    skewYSlider.value = String(skewY);
    skewYSlider.dispatchEvent(new Event('input'));
    syncingSidebar = false;
  }

  // Matches C++ controls: left mouse transforms (angle + scale), right mouse skews.
  function applyPointerState(buttonMask: number, x: number, y: number) {
    if (buttonMask & 1) {
      setFromTransformPoint(x, y);
    }
    if (buttonMask & 2) {
      skewX = x;
      skewY = y;
    }
    syncSidebar();
    draw();
  }

  canvas.addEventListener('pointerdown', e => {
    if (e.button !== 0 && e.button !== 2) return;
    canvas.setPointerCapture(e.pointerId);
    dragging = true;
    const p = pointerToAgg(e);
    const mask = e.buttons || (e.button === 2 ? 2 : 1);
    applyPointerState(mask, p.x, p.y);
    e.preventDefault();
  });
  canvas.addEventListener('contextmenu', e => e.preventDefault());
  canvas.addEventListener('pointermove', e => {
    if (!dragging) return;
    const p = pointerToAgg(e);
    applyPointerState(e.buttons, p.x, p.y);
    e.preventDefault();
  });
  canvas.addEventListener('pointerup', () => { dragging = false; });
  canvas.addEventListener('pointercancel', () => { dragging = false; });

  const canvasControls: CanvasControl[] = [];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag (bottom-left coords): set angle/scale. Right-drag: set skew from cursor position.';
  sidebar.appendChild(hint);

  syncSidebar();
  draw();
  return cleanupCC;
}
