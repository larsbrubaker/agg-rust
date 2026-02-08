import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Contour',
    'Letter "A" with adjustable contour width — matching C++ conv_contour.cpp.',
  );

  const W = 440, H = 330;
  let closeMode = 0;
  let contourWidth = 0;
  let autoDetect = 1;

  function draw() {
    renderToCanvas({
      demoName: 'conv_contour',
      canvas, width: W, height: H,
      params: [closeMode, contourWidth, autoDetect],
      timeDisplay: timeEl,
    });
  }

  const radioEls = addRadioGroup(sidebar, 'Close', ['Close', 'Close CW', 'Close CCW'], 0, v => { closeMode = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', -100, 100, 0, 1, v => { contourWidth = v; draw(); });
  const cbAuto = addCheckbox(sidebar, 'Auto-detect orientation', true, v => { autoDetect = v ? 1 : 0; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: v => { closeMode = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 430, y2: 22, min: -100, max: 100, sidebarEl: slWidth, onChange: v => { contourWidth = v; draw(); } },
    { type: 'checkbox', x1: 140, y1: 25, x2: 430, y2: 40, sidebarEl: cbAuto, onChange: v => { autoDetect = v ? 1 : 0; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
