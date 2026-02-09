import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Polar Transform',
    'Slider control warped through polar coordinates — matching C++ trans_polar.cpp.',
  );

  const W = 600, H = 400;
  let value = 32.0;
  let spiral = 0.0;
  let baseY = 120.0;

  function draw() {
    renderToCanvas({
      demoName: 'trans_polar',
      canvas, width: W, height: H,
      params: [value, spiral, baseY],
      timeDisplay: timeEl,
    });
  }

  const slVal = addSlider(sidebar, 'Value', 0, 100, 32, 1, v => { value = v; draw(); });
  const slSpiral = addSlider(sidebar, 'Spiral', -0.1, 0.1, 0, 0.001, v => { spiral = v; draw(); });
  const slBaseY = addSlider(sidebar, 'Base Y', 50, 200, 120, 1, v => { baseY = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 10, y1: 10, x2: 590, y2: 17, min: 0, max: 100, sidebarEl: slVal, onChange: v => { value = v; draw(); } },
    { type: 'slider', x1: 10, y1: 30, x2: 590, y2: 37, min: -0.1, max: 0.1, sidebarEl: slSpiral, onChange: v => { spiral = v; draw(); } },
    { type: 'slider', x1: 10, y1: 50, x2: 590, y2: 57, min: 50, max: 200, sidebarEl: slBaseY, onChange: v => { baseY = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
