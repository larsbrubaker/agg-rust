import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Circles',
    'Random anti-aliased circles — matching C++ circles.cpp.',
  );

  const W = 500, H = 500;
  let count = 200;
  let minR = 3;
  let maxR = 30;
  let seed = 12345;

  function draw() {
    renderToCanvas({
      demoName: 'circles',
      canvas, width: W, height: H,
      params: [count, minR, maxR, seed],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Count', 10, 1000, 200, 10, v => { count = v; draw(); });
  addSlider(sidebar, 'Min Radius', 1, 20, 3, 1, v => { minR = v; draw(); });
  addSlider(sidebar, 'Max Radius', 5, 80, 30, 1, v => { maxR = v; draw(); });
  addSlider(sidebar, 'Seed', 1, 99999, 12345, 1, v => { seed = v; draw(); });

  draw();
}
