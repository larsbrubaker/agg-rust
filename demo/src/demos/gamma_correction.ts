import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gamma Correction',
    'Concentric ellipses with gamma curve visualization — matching C++ gamma_correction.cpp.',
  );

  const W = 400, H = 320;
  let thickness = 1.0;
  let contrast = 1.0;
  let gamma = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gamma_correction',
      canvas, width: W, height: H,
      params: [thickness, contrast, gamma],
      timeDisplay: timeEl,
    });
  }

  const slThick = addSlider(sidebar, 'Thickness', 0.0, 3.0, 1.0, 0.1, v => { thickness = v; draw(); });
  const slContrast = addSlider(sidebar, 'Contrast', 0.0, 1.0, 1.0, 0.01, v => { contrast = v; draw(); });
  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 3.0, 1.0, 0.1, v => { gamma = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 395, y2: 11, min: 0, max: 3, sidebarEl: slThick, onChange: v => { thickness = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 395, y2: 26, min: 0, max: 1, sidebarEl: slContrast, onChange: v => { contrast = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: 395, y2: 41, min: 0.5, max: 3, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
