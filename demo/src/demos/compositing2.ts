import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

const COMP_OP_NAMES = [
  'Clear', 'Src', 'Dst', 'SrcOver', 'DstOver', 'SrcIn', 'DstIn',
  'SrcOut', 'DstOut', 'SrcAtop', 'DstAtop', 'Xor', 'Plus', 'Minus',
  'Multiply', 'Screen', 'Overlay', 'Darken', 'Lighten',
  'ColorDodge', 'ColorBurn', 'HardLight', 'SoftLight', 'Difference', 'Exclusion',
];

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Compositing 2',
    'Multiple overlapping circles blended with selected SVG compositing mode.',
  );
  const W = 512, H = 400;
  let compOp = 3, srcAlpha = 200, dstAlpha = 200;

  function draw() {
    renderToCanvas({ demoName: 'compositing2', canvas, width: W, height: H,
      params: [compOp, srcAlpha, dstAlpha], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Src Alpha', 0, 255, 200, 1, v => { srcAlpha = v; draw(); });
  addSlider(sidebar, 'Dst Alpha', 0, 255, 200, 1, v => { dstAlpha = v; draw(); });

  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  radioDiv.style.maxHeight = '200px';
  radioDiv.style.overflowY = 'auto';
  const lbl = document.createElement('label');
  lbl.className = 'control-label';
  lbl.textContent = 'Comp Op';
  radioDiv.appendChild(lbl);
  COMP_OP_NAMES.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block'; row.style.cursor = 'pointer'; row.style.marginBottom = '1px';
    const rb = document.createElement('input');
    rb.type = 'radio'; rb.name = 'comp_op2'; rb.value = String(i);
    rb.checked = i === compOp;
    rb.addEventListener('change', () => { compOp = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  draw();
}
