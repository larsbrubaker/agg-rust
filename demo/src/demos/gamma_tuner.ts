import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gamma Tuner',
    'Gradient background + alpha pattern with gamma correction — matching C++ gamma_tuner.cpp.',
  );

  const W = 500, H = 500;
  let gamma = 2.2;
  let r = 1.0, g = 1.0, b = 1.0;
  let pattern = 2; // 0=horiz, 1=vert, 2=checkered

  function draw() {
    renderToCanvas({
      demoName: 'gamma_tuner',
      canvas, width: W, height: H,
      params: [gamma, r, g, b, pattern],
      timeDisplay: timeEl,
    });
  }

  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 4.0, 2.2, 0.01, v => { gamma = v; draw(); });
  const slR = addSlider(sidebar, 'R', 0, 1, 1, 0.01, v => { r = v; draw(); });
  const slG = addSlider(sidebar, 'G', 0, 1, 1, 0.01, v => { g = v; draw(); });
  const slB = addSlider(sidebar, 'B', 0, 1, 1, 0.01, v => { b = v; draw(); });
  const radioEls = addRadioGroup(sidebar, 'Pattern', ['Horizontal', 'Vertical', 'Checkered'], 2, v => { pattern = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 345, y2: 11, min: 0.5, max: 4, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 345, y2: 26, min: 0, max: 1, sidebarEl: slR, onChange: v => { r = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: 345, y2: 41, min: 0, max: 1, sidebarEl: slG, onChange: v => { g = v; draw(); } },
    { type: 'slider', x1: 5, y1: 50, x2: 345, y2: 56, min: 0, max: 1, sidebarEl: slB, onChange: v => { b = v; draw(); } },
    { type: 'radio', x1: 355, y1: 1, x2: 495, y2: 60, numItems: 3, sidebarEls: radioEls, onChange: v => { pattern = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
