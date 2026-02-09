import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Alpha Mask 2',
    'Alpha mask with random ellipses modulating lion rendering — matching C++ alpha_mask2.cpp.',
  );
  const W = 512, H = 400;
  let numEllipses = 50, angle = 0, scale = 1.0;

  function draw() {
    renderToCanvas({ demoName: 'alpha_mask2', canvas, width: W, height: H,
      params: [numEllipses, angle, scale], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Mask Ellipses', 5, 200, 50, 1, v => { numEllipses = v; draw(); });
  addSlider(sidebar, 'Rotation', -180, 180, 0, 1, v => { angle = v; draw(); });
  addSlider(sidebar, 'Scale', 0.3, 3.0, 1.0, 0.05, v => { scale = v; draw(); });

  draw();
}
