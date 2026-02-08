import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Mask',
    'Lion with elliptical alpha mask — matching C++ alpha_mask.cpp.',
  );

  const W = 512, H = 400;

  let angle = Math.PI;
  let scale = 1.0;
  let skewX = 0;
  let skewY = 0;
  let dragging = false;
  let lastX = 0, lastY = 0;
  let rightDrag = false;

  function draw() {
    renderToCanvas({
      demoName: 'alpha_mask',
      canvas, width: W, height: H,
      params: [angle, scale, skewX, skewY],
      timeDisplay: timeEl,
    });
  }

  // Left-drag = rotate + scale, right-drag = skew
  canvas.addEventListener('mousedown', e => {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    lastX = (e.clientX - rect.left) * sx;
    lastY = (e.clientY - rect.top) * sx;
    dragging = true;
    rightDrag = e.button === 2;
  });
  canvas.addEventListener('contextmenu', e => e.preventDefault());
  canvas.addEventListener('mousemove', e => {
    if (!dragging) return;
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const x = (e.clientX - rect.left) * sx;
    const y = (e.clientY - rect.top) * sx;
    const dx = x - lastX;
    const dy = y - lastY;
    if (rightDrag) {
      skewX += dx;
      skewY += dy;
    } else {
      angle += dy * 0.01;
      scale += dx * 0.005;
      if (scale < 0.01) scale = 0.01;
    }
    lastX = x;
    lastY = y;
    draw();
  });
  canvas.addEventListener('mouseup', () => { dragging = false; });
  canvas.addEventListener('mouseleave', () => { dragging = false; });

  const canvasControls: CanvasControl[] = [];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: rotate + scale. Right-drag: skew. Lion is masked by random ellipses.';
  sidebar.appendChild(hint);

  draw();
  return cleanupCC;
}
