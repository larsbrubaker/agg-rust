import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

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

  const slAngle = addSlider(sidebar, 'Angle', -180, 180, 0, 1, v => { angle = v; draw(); });
  const slScale = addSlider(sidebar, 'Scale', 0.1, 5.0, 1.0, 0.05, v => { scale = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 300, y2: 12, min: -180, max: 180, sidebarEl: slAngle, onChange: v => { angle = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 300, y2: 27, min: 0.1, max: 5, sidebarEl: slScale, onChange: v => { scale = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
