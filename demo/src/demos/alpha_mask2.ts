import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Alpha Mask 2',
    'Alpha mask with random ellipses modulating lion rendering — matching C++ alpha_mask2.cpp.',
  );
  const W = 512, H = 400;
  let numEllipses = 10;
  let angle = 0.0;
  let scale = 1.0;
  let skewX = 0.0;
  let skewY = 0.0;
  let dragging = false;

  function draw() {
    renderToCanvas({ demoName: 'alpha_mask2', canvas, width: W, height: H,
      params: [numEllipses, angle, scale, skewX, skewY], timeDisplay: timeEl });
  }

  function canvasPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTop = (e.clientY - rect.top) * sy;
    // C++ demo runs with flip_y=true, so we use bottom-left coordinates.
    return { x, y: H - yTop };
  }

  function transform(x: number, y: number) {
    const dx = x - W / 2;
    const dy = y - H / 2;
    angle = Math.atan2(dy, dx);
    scale = Math.hypot(dx, dy) / 100.0;
    if (scale < 0.01) scale = 0.01;
  }

  function applyPointer(flags: number, x: number, y: number) {
    // Match on_mouse_button_down behavior from C++:
    // left sets angle/scale, right sets skew.
    if ((flags & 1) !== 0) {
      transform(x, y);
    }
    if ((flags & 2) !== 0) {
      skewX = x;
      skewY = y;
    }
  }

  const slNum = addSlider(sidebar, 'N', 5, 100, 10, 1, v => {
    numEllipses = Math.round(v);
    draw();
  });

  const canvasControls: CanvasControl[] = [
    {
      type: 'slider',
      x1: 5, y1: 5, x2: 150, y2: 12,
      min: 5, max: 100,
      sidebarEl: slNum,
      onChange: v => {
        numEllipses = Math.round(v);
        draw();
      },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  function onPointerDown(e: PointerEvent) {
    const p = canvasPos(e);
    canvas.setPointerCapture(e.pointerId);
    dragging = true;
    if (e.button === 2) {
      applyPointer(2, p.x, p.y);
      draw();
      e.preventDefault();
      return;
    }
    if (e.button === 0) {
      applyPointer(1, p.x, p.y);
      draw();
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const p = canvasPos(e);
    let flags = 0;
    if ((e.buttons & 1) !== 0) flags |= 1;
    if ((e.buttons & 2) !== 0) flags |= 2;
    if (flags === 0) return;
    applyPointer(flags, p.x, p.y);
    draw();
  }

  function onPointerUp() {
    dragging = false;
  }

  function onContextMenu(e: Event) {
    e.preventDefault();
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);
  canvas.addEventListener('contextmenu', onContextMenu);
  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}
