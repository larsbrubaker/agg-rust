import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Rasterizer Compound',
    'Compound rasterizer with layer order control — matching C++ rasterizer_compound.cpp.',
  );
  const W = 512, H = 400;
  let strokeWidth = 2.0, invertOrder = 0;

  function draw() {
    renderToCanvas({ demoName: 'rasterizer_compound', canvas, width: W, height: H,
      params: [strokeWidth, invertOrder], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Stroke Width', 0.5, 10, 2, 0.1, v => { strokeWidth = v; draw(); });

  const cbDiv = document.createElement('div');
  cbDiv.className = 'control-group';
  const cb = document.createElement('input');
  cb.type = 'checkbox'; cb.id = 'rc_invert'; cb.checked = false;
  cb.addEventListener('change', () => { invertOrder = cb.checked ? 1 : 0; draw(); });
  const cbLabel = document.createElement('label');
  cbLabel.htmlFor = cb.id; cbLabel.textContent = ' Invert Z-Order';
  cbDiv.appendChild(cb); cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);

  draw();
}
