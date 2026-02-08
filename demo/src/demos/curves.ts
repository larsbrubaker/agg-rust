import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Curves',
    'Quadratic and cubic Bezier curves with control points.',
  );

  // Control points normalized 0-1
  let p1y = 0.1;
  let p2y = 0.1;
  const W = 700, H = 500;

  function draw() {
    renderToCanvas({
      demoName: 'curves',
      canvas, width: W, height: H,
      params: [0.1, 0.8, 0.3, p1y, 0.7, p2y, 0.9, 0.8],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Control Point 1 Y', 0.0, 1.0, 0.1, 0.01, v => { p1y = v; draw(); });
  addSlider(sidebar, 'Control Point 2 Y', 0.0, 1.0, 0.1, 0.01, v => { p2y = v; draw(); });

  draw();
}
