import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupRotateScale } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Lion',
    'The classic AGG vector lion — left-drag to rotate & scale, right-drag to skew.',
  );

  let angle = 0;
  let scale = 1.0;
  let skewX = 0;
  let skewY = 0;
  let alpha = 26;
  const W = 512, H = 400;

  function draw() {
    renderToCanvas({
      demoName: 'lion',
      canvas, width: W, height: H,
      params: [angle, scale, skewX, skewY, alpha],
      timeDisplay: timeEl,
    });
  }

  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => { angle = a; scale = s; draw(); },
    onRightDrag: (x, y) => { skewX = x; skewY = y; draw(); },
  });

  const slAlpha = addSlider(sidebar, 'Alpha', 0, 255, 26, 1, v => { alpha = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 507, y2: 12, min: 0, max: 255, sidebarEl: slAlpha, onChange: v => { alpha = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: rotate & scale. Right-drag: skew.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupRS(); cleanupCC(); };
}
