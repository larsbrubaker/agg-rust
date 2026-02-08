import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gamma Correction',
    'Concentric ellipses with gamma curve visualization.',
  );

  const W = 500, H = 400;
  let thickness = 1.0;
  let gamma = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gamma_correction',
      canvas, width: W, height: H,
      params: [thickness, gamma],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Thickness', 0.1, 3.0, 1.0, 0.1, v => { thickness = v; draw(); });
  addSlider(sidebar, 'Gamma', 0.1, 3.0, 1.0, 0.1, v => { gamma = v; draw(); });

  draw();
}
