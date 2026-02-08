import { createDemoLayout, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupGradientDrag } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gradients',
    '6 gradient types with mouse interaction — matching C++ gradients.cpp.',
  );

  const W = 600, H = 600;

  let cx = W / 2;
  let cy = H / 2;
  let angle = 0;
  let scale = 1.0;
  let gradType = 0;
  const scaleX = 1.0;
  const scaleY = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gradients',
      canvas, width: W, height: H,
      params: [cx, cy, angle, scale, gradType, scaleX, scaleY],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupGradientDrag({
    canvas,
    centerX: cx,
    centerY: cy,
    angle,
    scale,
    onUpdate: (newCx, newCy, newAngle, newScale) => {
      cx = newCx;
      cy = newCy;
      angle = newAngle;
      scale = newScale;
      draw();
    },
  });

  addRadioGroup(sidebar, 'Gradient Type',
    ['Radial', 'Diamond', 'Linear', 'XY', 'Sqrt XY', 'Conic'],
    0,
    v => { gradType = v; draw(); },
  );

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: move center. Right-drag: rotate & scale.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
