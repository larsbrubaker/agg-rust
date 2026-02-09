import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Blend Color',
    'Shape with blurred shadow — demonstrates blur compositing matching C++ blend_color.cpp.',
  );

  const W = 440, H = 330;
  let blurRadius = 15.0;

  function draw() {
    renderToCanvas({
      demoName: 'blend_color',
      canvas, width: W, height: H,
      params: [blurRadius, 10, 10],
      timeDisplay: timeEl,
    });
  }

  const slBlur = addSlider(sidebar, 'Blur Radius', 0, 40, 15, 0.5, v => { blurRadius = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 435, y2: 12, min: 0, max: 40, sidebarEl: slBlur, onChange: v => { blurRadius = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
