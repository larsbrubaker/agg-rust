import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';

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

  addRadioGroup(sidebar, 'Close', ['Close', 'Close CW', 'Close CCW'], 0, v => { closeMode = v; draw(); });
  addSlider(sidebar, 'Width', -100, 100, 0, 1, v => { contourWidth = v; draw(); });
  addCheckbox(sidebar, 'Auto-detect orientation', true, v => { autoDetect = v ? 1 : 0; draw(); });

  draw();
}
