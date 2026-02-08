import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupRotateScale } from '../mouse-helpers.ts';

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
  let alpha = 255;
  const W = 600, H = 600;

  function draw() {
    renderToCanvas({
      demoName: 'lion',
      canvas, width: W, height: H,
      params: [angle, scale, skewX, skewY, alpha],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => { angle = a; scale = s; draw(); },
    onRightDrag: (x, y) => { skewX = x; skewY = y; draw(); },
  });

  addSlider(sidebar, 'Alpha', 0, 255, 255, 1, v => { alpha = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: rotate & scale. Right-drag: skew.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
