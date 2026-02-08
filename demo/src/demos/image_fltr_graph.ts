import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Filter Graph',
    'Visualization of image filter weight functions — matching C++ image_fltr_graph.cpp.',
  );

  const W = 780, H = 300;
  let filterIdx = 0;
  let radius = 4.0;

  function draw() {
    renderToCanvas({
      demoName: 'image_fltr_graph',
      canvas, width: W, height: H,
      params: [filterIdx, radius],
      timeDisplay: timeEl,
    });
  }

  addRadioGroup(sidebar, 'Filter', [
    'Bilinear', 'Bicubic', 'Spline16', 'Spline36',
  ], 0, v => { filterIdx = v; draw(); });

  // Second group of filters
  const group2 = document.createElement('div');
  group2.className = 'control-radio-group';
  const label2 = document.createElement('div');
  label2.style.cssText = 'font-size:11px;color:var(--text-muted);margin-bottom:4px;';
  label2.textContent = 'More Filters';
  group2.appendChild(label2);

  const filters2 = ['Hanning', 'Hamming', 'Hermite', 'Kaiser', 'Quadric', 'Catrom', 'Gaussian', 'Bessel', 'Mitchell', 'Sinc', 'Lanczos', 'Blackman'];
  filters2.forEach((name, i) => {
    const lbl = document.createElement('label');
    lbl.className = 'control-radio-label';
    const radio = document.createElement('input');
    radio.type = 'radio';
    radio.name = 'filter2';
    radio.value = String(i + 4);
    radio.addEventListener('change', () => {
      filterIdx = i + 4;
      sidebar.querySelectorAll('input[name="Filter"]').forEach((el: any) => el.checked = false);
      draw();
    });
    lbl.appendChild(radio);
    lbl.appendChild(document.createTextNode(' ' + name));
    group2.appendChild(lbl);
  });
  sidebar.appendChild(group2);

  const slRadius = addSlider(sidebar, 'Radius (Sinc/Lanczos/Blackman)', 2, 8, 4, 0.5, v => { radius = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: W - 5, y2: 10, min: 2, max: 8, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
