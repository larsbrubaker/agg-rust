import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Component Rendering',
    'Three overlapping circles rendered to separate R/G/B gray channels — matching C++ component_rendering.cpp.',
  );

  const W = 440, H = 330;
  let alpha = 255;

  function draw() {
    renderToCanvas({
      demoName: 'component_rendering',
      canvas, width: W, height: H,
      params: [alpha],
      timeDisplay: timeEl,
    });
  }

  const slAlpha = addSlider(sidebar, 'Alpha', 0, 255, 255, 1, v => { alpha = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 435, y2: 12, min: 0, max: 255, sidebarEl: slAlpha, onChange: v => { alpha = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
