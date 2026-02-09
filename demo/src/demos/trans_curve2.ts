import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Text Along Curve 2',
    'Text warped along a second curve variant using trans_single_path — matching C++ trans_curve2_test.cpp.',
  );

  const W = 600, H = 600;
  let numPoints = 200;

  function draw() {
    renderToCanvas({
      demoName: 'trans_curve2',
      canvas, width: W, height: H,
      params: [numPoints],
      timeDisplay: timeEl,
    });
  }

  const slPoints = addSlider(sidebar, 'Num Points', 10, 400, 200, 10, v => { numPoints = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 595, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: v => { numPoints = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
