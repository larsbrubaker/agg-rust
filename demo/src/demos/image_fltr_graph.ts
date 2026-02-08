import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Filter Graph',
    'Image filter weight function visualization — matching C++ image_fltr_graph.cpp.',
  );

  const W = 780, H = 300;
  let radius = 4.0;

  // 16 filter checkboxes matching C++ order
  const filterNames = [
    'bilinear', 'bicubic', 'spline16', 'spline36',
    'hanning', 'hamming', 'hermite', 'kaiser',
    'quadric', 'catrom', 'gaussian', 'bessel',
    'mitchell', 'sinc', 'lanczos', 'blackman',
  ];
  const enabled: boolean[] = new Array(16).fill(false);

  function draw() {
    renderToCanvas({
      demoName: 'image_fltr_graph',
      canvas, width: W, height: H,
      params: [radius, ...enabled.map(v => v ? 1 : 0)],
      timeDisplay: timeEl,
    });
  }

  const slRadius = addSlider(sidebar, 'Radius', 2, 8, 4, 0.001, v => { radius = v; draw(); });

  // 16 checkboxes
  const cbEls: HTMLInputElement[] = [];
  for (let i = 0; i < 16; i++) {
    const cb = addCheckbox(sidebar, filterNames[i], false, v => { enabled[i] = v; draw(); });
    cbEls.push(cb);
  }

  // Canvas controls: slider at top, 16 checkboxes along left
  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 775, y2: 15, min: 2, max: 8, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
  ];
  for (let i = 0; i < 16; i++) {
    const y = 30 + 15 * i;
    canvasControls.push({
      type: 'checkbox', x1: 8, y1: y, x2: 120, y2: y + 12,
      sidebarEl: cbEls[i], onChange: v => { enabled[i] = v > 0.5; draw(); },
    });
  }
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Enable filters to compare weight curves: red=weight, green=cumulative, blue=normalized.';
  sidebar.appendChild(hint);

  draw();
  return cleanupCC;
}
