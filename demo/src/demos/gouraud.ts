import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gouraud Shading',
    'Smooth color interpolation across triangles.',
  );

  let xOffset = 0;
  const W = 700, H = 500;

  function draw() {
    renderToCanvas({
      demoName: 'gouraud',
      canvas, width: W, height: H,
      params: [xOffset],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'X Offset', -100, 100, 0, 1, v => { xOffset = v; draw(); });

  draw();
}
