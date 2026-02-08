import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Lion',
    'The classic AGG vector lion with rotation and scaling.',
  );

  let angle = 0;
  let scale = 1.0;
  const W = 600, H = 600;

  function draw() {
    renderToCanvas({
      demoName: 'lion',
      canvas, width: W, height: H,
      params: [angle, scale],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Rotation', -180, 180, 0, 1, v => { angle = v; draw(); });
  addSlider(sidebar, 'Scale', 0.2, 4.0, 1.0, 0.1, v => { scale = v; draw(); });

  draw();
}
