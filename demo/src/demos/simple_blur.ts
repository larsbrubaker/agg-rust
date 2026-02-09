import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupRotateScale } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Simple Blur',
    'Lion with 3×3 box blur — left half original, right half blurred. Matching C++ simple_blur.cpp.',
  );

  const W = 512, H = 400;
  let angle = 0;
  let scale = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'simple_blur',
      canvas, width: W, height: H,
      params: [angle, scale],
      timeDisplay: timeEl,
    });
  }

  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => { angle = a; scale = s; draw(); },
  });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: rotate & scale.';
  sidebar.appendChild(hint);

  draw();
  return cleanupRS;
}
