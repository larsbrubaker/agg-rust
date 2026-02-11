import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Circles',
    'Random anti-aliased circles — matching C++ circles.cpp.',
  );

  const W = 400, H = 400;
  let zMin = 0.3;
  let zMax = 0.7;
  let size = 0.5;
  let selectivity = 0.5;
  let seed = 1;

  function draw() {
    renderToCanvas({
      demoName: 'circles',
      canvas, width: W, height: H,
      params: [zMin, zMax, size, selectivity, seed],
      timeDisplay: timeEl,
    });
  }

  const slZMin = addSlider(sidebar, 'Z Min', 0, 1, zMin, 0.01, v => {
    zMin = Math.min(v, zMax - 0.01);
    draw();
  });
  const slZMax = addSlider(sidebar, 'Z Max', 0, 1, zMax, 0.01, v => {
    zMax = Math.max(v, zMin + 0.01);
    draw();
  });
  const slSize = addSlider(sidebar, 'Size', 0, 1, size, 0.01, v => { size = v; draw(); });
  const slSel = addSlider(sidebar, 'Selectivity', 0, 1, selectivity, 0.01, v => { selectivity = v; draw(); });
  addSlider(sidebar, 'Seed', 1, 99999, seed, 1, v => { seed = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'scale', x1: 5, y1: 5, x2: W - 5, y2: 12, min: 0, max: 1, minDelta: 0.01, sidebarEl1: slZMin, sidebarEl2: slZMax },
    { type: 'slider', x1: 5, y1: 20, x2: W - 5, y2: 27, min: 0, max: 1, sidebarEl: slSel, onChange: v => { selectivity = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: W - 5, y2: 42, min: 0, max: 1, sidebarEl: slSize, onChange: v => { size = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return () => cleanupCC();
}
