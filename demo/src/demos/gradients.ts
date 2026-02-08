import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gradients',
    'Linear and radial gradient fills with multi-stop color interpolation.',
  );

  let angle = 0;
  const W = 700, H = 600;

  function draw() {
    renderToCanvas({
      demoName: 'gradients',
      canvas, width: W, height: H,
      params: [angle],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Gradient Rotation', -180, 180, 0, 1, v => { angle = v; draw(); });

  draw();
}
