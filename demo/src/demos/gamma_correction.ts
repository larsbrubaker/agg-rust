import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gamma Correction',
    'Concentric ellipses with gamma curve visualization — matching C++ gamma_correction.cpp.',
  );

  const W = 400, H = 320;
  let thickness = 1.0;
  let contrast = 1.0;
  let gamma = 1.0;
  let rx = W / 3;
  let ry = H / 3;

  function draw() {
    renderToCanvas({
      demoName: 'gamma_correction',
      canvas, width: W, height: H,
      params: [thickness, contrast, gamma, rx, ry],
      timeDisplay: timeEl,
    });
  }

  const slThick = addSlider(sidebar, 'Thickness', 0.0, 3.0, 1.0, 0.1, v => { thickness = v; draw(); });
  const slContrast = addSlider(sidebar, 'Contrast', 0.0, 1.0, 1.0, 0.01, v => { contrast = v; draw(); });
  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 3.0, 1.0, 0.1, v => { gamma = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 395, y2: 11, min: 0, max: 3, sidebarEl: slThick, onChange: v => { thickness = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 395, y2: 26, min: 0.5, max: 3, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: 395, y2: 41, min: 0, max: 1, sidebarEl: slContrast, onChange: v => { contrast = v; draw(); } },
  ];

  // Match C++ on_mouse_button_down/on_mouse_move:
  // m_rx = abs(width/2 - x), m_ry = abs(height/2 - y), with bottom-left coords.
  function aggPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const y = canvas.height - (e.clientY - rect.top) * scaleY;
    return { x, y };
  }

  function updateRadiiFromPointer(e: PointerEvent): void {
    const pos = aggPos(e);
    rx = Math.abs(W * 0.5 - pos.x);
    ry = Math.abs(H * 0.5 - pos.y);
    draw();
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0 || e.defaultPrevented) return;
    canvas.setPointerCapture(e.pointerId);
    updateRadiiFromPointer(e);
  }

  function onPointerMove(e: PointerEvent) {
    if ((e.buttons & 1) === 0 || e.defaultPrevented) return;
    updateRadiiFromPointer(e);
  }

  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);

  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
  };
}
