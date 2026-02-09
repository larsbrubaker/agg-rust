import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Raster Text',
    'All 34 embedded bitmap fonts rendered with sample text — matching C++ raster_text.cpp.',
  );

  const W = 640, H = 480;

  function draw() {
    renderToCanvas({
      demoName: 'raster_text',
      canvas, width: W, height: H,
      params: [],
      timeDisplay: timeEl,
    });
  }

  draw();
}
