import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Image Resample',
    'Image resampling with affine and perspective transforms — matching C++ image_resample.cpp.',
  );
  const W = 512, H = 400;
  let mode = 0, blur = 1.0;

  function draw() {
    renderToCanvas({ demoName: 'image_resample', canvas, width: W, height: H,
      params: [mode, blur], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Mode (0-3)', 0, 3, 0, 1, v => { mode = v; draw(); });
  addSlider(sidebar, 'Blur', 0.5, 2.0, 1.0, 0.05, v => { blur = v; draw(); });

  draw();
}
