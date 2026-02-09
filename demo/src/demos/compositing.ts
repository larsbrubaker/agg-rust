import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

const COMP_OP_NAMES = [
  'Clear', 'Src', 'Dst', 'SrcOver', 'DstOver', 'SrcIn', 'DstIn',
  'SrcOut', 'DstOut', 'SrcAtop', 'DstAtop', 'Xor', 'Plus', 'Minus',
  'Multiply', 'Screen', 'Overlay', 'Darken', 'Lighten',
  'ColorDodge', 'ColorBurn', 'HardLight', 'SoftLight', 'Difference', 'Exclusion',
];

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Compositing',
    'SVG compositing operations — two shapes blended with selectable comp_op mode.',
  );
  const W = 512, H = 400;
  let compOp = 3, srcAlpha = 255, dstAlpha = 255;

  function draw() {
    renderToCanvas({ demoName: 'compositing', canvas, width: W, height: H,
      params: [compOp, srcAlpha, dstAlpha], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Src Alpha', 0, 255, 255, 1, v => { srcAlpha = v; draw(); });
  addSlider(sidebar, 'Dst Alpha', 0, 255, 255, 1, v => { dstAlpha = v; draw(); });

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
    rb.type = 'radio'; rb.name = 'comp_op'; rb.value = String(i);
    rb.checked = i === compOp;
    rb.addEventListener('change', () => { compOp = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  draw();
}
