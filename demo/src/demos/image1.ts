import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Affine Transforms',
    'Original AGG spheres image rotated/scaled through an ellipse with bilinear filtering. Port of image1.cpp.',
  );

  const W = 340, H = 360;
  let angle = 0;
  let scale = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'image1',
      canvas, width: W, height: H,
      params: [angle, scale],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Angle', -180, 180, 0, 1, v => { angle = v; draw(); });
  addSlider(sidebar, 'Scale', 0.1, 5.0, 1.0, 0.05, v => { scale = v; draw(); });

  draw();
}
