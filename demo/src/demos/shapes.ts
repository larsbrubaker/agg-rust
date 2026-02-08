import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Shapes',
    'Anti-aliased circles, ellipses, and rounded rectangles.',
  );

  let count = 8;
  const W = 800, H = 500;

  function draw() {
    renderToCanvas({
      demoName: 'shapes',
      canvas, width: W, height: H,
      params: [count],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Shape Count', 2, 16, 8, 1, v => { count = v; draw(); });

  draw();
}
