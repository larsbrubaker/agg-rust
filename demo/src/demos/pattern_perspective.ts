import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Pattern Perspective',
    'Perspective-transformed pattern fill in a draggable quad — matching C++ pattern_perspective.cpp.',
  );

  const W = 600, H = 600;

  const vertices: Vertex[] = [
    { x: W * 0.17, y: H * 0.17 },
    { x: W * 0.83, y: H * 0.08 },
    { x: W * 0.83, y: H * 0.83 },
    { x: W * 0.17, y: H * 0.83 },
  ];

  let transType = 0;

  function draw() {
    renderToCanvas({
      demoName: 'pattern_perspective',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        transType,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw,
  });

  // Radio buttons for transform type
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Transform Type';
  radioDiv.appendChild(radioLabel);
  const names = ['Affine', 'Bilinear', 'Perspective'];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'pat_persp_trans';
    rb.value = String(i);
    rb.checked = i === transType;
    rb.addEventListener('change', () => { transType = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 quad corners to transform the pattern.';
  sidebar.appendChild(hint);

  draw();
  return cleanupDrag;
}
