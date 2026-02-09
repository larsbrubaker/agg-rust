import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Scanline Boolean 2',
    'Complex shapes combined with boolean operations — matching C++ scanline_boolean2.cpp.',
  );

  const W = 512, H = 400;
  let testCase = 0;
  let operation = 0;

  function draw() {
    renderToCanvas({
      demoName: 'scanline_boolean2',
      canvas, width: W, height: H,
      params: [testCase, operation],
      timeDisplay: timeEl,
    });
  }

  // Test case radio buttons
  const caseDiv = document.createElement('div');
  caseDiv.className = 'control-group';
  const caseLabel = document.createElement('label');
  caseLabel.className = 'control-label';
  caseLabel.textContent = 'Test Case';
  caseDiv.appendChild(caseLabel);
  const caseNames = ['Ellipses', 'Rectangles', 'Star & Circle', 'Stroke & Triangle'];
  caseNames.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'sbool2_case';
    rb.value = String(i);
    rb.checked = i === testCase;
    rb.addEventListener('change', () => { testCase = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    caseDiv.appendChild(row);
  });
  sidebar.appendChild(caseDiv);

  // Boolean operation radio buttons
  const opDiv = document.createElement('div');
  opDiv.className = 'control-group';
  const opLabel = document.createElement('label');
  opLabel.className = 'control-label';
  opLabel.textContent = 'Boolean Operation';
  opDiv.appendChild(opLabel);
  const opNames = ['OR', 'AND', 'XOR', 'A - B', 'B - A'];
  opNames.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'sbool2_op';
    rb.value = String(i);
    rb.checked = i === operation;
    rb.addEventListener('change', () => { operation = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    opDiv.appendChild(row);
  });
  sidebar.appendChild(opDiv);

  draw();
}
