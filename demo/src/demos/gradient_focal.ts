import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gradient Focal',
    'Radial gradient with moveable focal point — matching C++ gradient_focal.cpp.',
  );

  const W = 600, H = 400;
  let focalX = W / 2;
  let focalY = H / 2;
  let gamma = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gradient_focal',
      canvas, width: W, height: H,
      params: [focalX, focalY, gamma],
      timeDisplay: timeEl,
    });
  }

  // Mouse drag moves the focal point
  let dragging = false;
  function aggPos(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: canvas.height - (e.clientY - rect.top) * sy,
    };
  }
  function onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    dragging = true;
    canvas.setPointerCapture(e.pointerId);
    const p = aggPos(e);
    focalX = p.x; focalY = p.y;
    draw();
  }
  function onMove(e: PointerEvent) {
    if (!dragging) return;
    const p = aggPos(e);
    focalX = p.x; focalY = p.y;
    draw();
  }
  function onUp() { dragging = false; }

  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);

  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 2.5, 1.0, 0.01, v => { gamma = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 340, y2: 12, min: 0.5, max: 2.5, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Click/drag to move the focal point.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onDown);
    canvas.removeEventListener('pointermove', onMove);
    canvas.removeEventListener('pointerup', onUp);
    canvas.removeEventListener('pointercancel', onUp);
    cleanupCC();
  };
}
