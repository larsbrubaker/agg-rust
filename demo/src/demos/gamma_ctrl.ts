import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gamma Control',
    'Interactive gamma spline widget with stroked ellipses — matching C++ gamma_ctrl.cpp.',
  );

  const W = 500, H = 400;
  let kx1 = 1.0, ky1 = 1.0, kx2 = 1.0, ky2 = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gamma_ctrl',
      canvas, width: W, height: H,
      params: [kx1, ky1, kx2, ky2],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'kx1', 0.001, 1.999, 1.0, 0.01, v => { kx1 = v; draw(); });
  addSlider(sidebar, 'ky1', 0.001, 1.999, 1.0, 0.01, v => { ky1 = v; draw(); });
  addSlider(sidebar, 'kx2', 0.001, 1.999, 1.0, 0.01, v => { kx2 = v; draw(); });
  addSlider(sidebar, 'ky2', 0.001, 1.999, 1.0, 0.01, v => { ky2 = v; draw(); });

  draw();
}
