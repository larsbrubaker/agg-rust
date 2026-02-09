import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Scanline Boolean',
    'Two overlapping circle groups combined with boolean operations — matching C++ scanline_boolean.cpp.',
  );

  const W = 512, H = 400;
  let operation = 0;

  function draw() {
    renderToCanvas({
      demoName: 'scanline_boolean',
      canvas, width: W, height: H,
      params: [operation],
      timeDisplay: timeEl,
    });
  }

  // Radio buttons for boolean operation
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Boolean Operation';
  radioDiv.appendChild(radioLabel);
  const names = ['OR (Union)', 'AND (Intersect)', 'XOR', 'A - B', 'B - A'];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'sbool_op';
    rb.value = String(i);
    rb.checked = i === operation;
    rb.addEventListener('change', () => { operation = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  draw();
}
