import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Filters 2',
    '4x4 test image filtered through 17 filter types — matching C++ image_filters2.cpp.',
  );

  const W = 500, H = 340;
  let filterIdx = 1;
  let gamma = 1.0;
  let radius = 4.0;
  let normalize = true;

  const filterNames = [
    'simple (NN)', 'bilinear', 'bicubic', 'spline16', 'spline36',
    'hanning', 'hamming', 'hermite', 'kaiser', 'quadric',
    'catrom', 'gaussian', 'bessel', 'mitchell', 'sinc',
    'lanczos', 'blackman',
  ];

  function draw() {
    renderToCanvas({
      demoName: 'image_filters2',
      canvas, width: W, height: H,
      params: [filterIdx, gamma, radius, normalize ? 1 : 0],
      timeDisplay: timeEl,
    });
  }

  const radioEls = addRadioGroup(sidebar, 'Filter', filterNames, 1, v => { filterIdx = v; draw(); });
  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 3.0, 1.0, 0.001, v => { gamma = v; draw(); });
  const slRadius = addSlider(sidebar, 'Filter Radius', 2, 8, 4, 0.001, v => { radius = v; draw(); });
  const cbNormalize = addCheckbox(sidebar, 'Normalize Filter', true, v => { normalize = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 0, y1: 0, x2: 110, y2: 210, numItems: 17, sidebarEls: radioEls, onChange: v => { filterIdx = v; draw(); } },
    { type: 'slider', x1: 115, y1: 5, x2: 495, y2: 11, min: 0.5, max: 3, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
    { type: 'slider', x1: 115, y1: 20, x2: 495, y2: 26, min: 2, max: 8, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
    { type: 'checkbox', x1: 8, y1: 215, x2: 180, y2: 228, sidebarEl: cbNormalize, onChange: v => { normalize = v > 0.5; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Select a filter to see the 4x4 test image scaled to 300x300. Filters 14+ use the radius slider.';
  sidebar.appendChild(hint);

  draw();
  return cleanupCC;
}
