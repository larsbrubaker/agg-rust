import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Pattern Fill',
    'Star polygon filled with a repeating tile pattern — matching C++ pattern_fill.cpp.',
  );

  const W = 512, H = 400;
  let patSize = 30;
  let polyAngle = 0;

  function draw() {
    renderToCanvas({
      demoName: 'pattern_fill',
      canvas, width: W, height: H,
      params: [patSize, polyAngle],
      timeDisplay: timeEl,
    });
  }

  const slSize = addSlider(sidebar, 'Pattern Size', 10, 60, 30, 1, v => { patSize = v; draw(); });
  const slAngle = addSlider(sidebar, 'Polygon Angle', -180, 180, 0, 1, v => { polyAngle = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 507, y2: 12, min: 10, max: 60, sidebarEl: slSize, onChange: v => { patSize = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 507, y2: 27, min: -180, max: 180, sidebarEl: slAngle, onChange: v => { polyAngle = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
