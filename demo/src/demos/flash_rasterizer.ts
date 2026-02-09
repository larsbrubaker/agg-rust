import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Flash Rasterizer',
    'Compound rasterizer with multi-style filled shapes — adapted from C++ flash_rasterizer.cpp.',
  );
  const W = 512, H = 400;
  let scale = 1.0, rotation = 0;

  function draw() {
    renderToCanvas({ demoName: 'flash_rasterizer', canvas, width: W, height: H,
      params: [scale, rotation], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Scale', 0.2, 3, 1, 0.01, v => { scale = v; draw(); });
  addSlider(sidebar, 'Rotation', -180, 180, 0, 1, v => { rotation = v; draw(); });
  draw();
}
