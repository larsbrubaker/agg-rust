import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Perspective',
    'Image transformed through affine/bilinear/perspective quad — matching C++ image_perspective.cpp.',
  );

  const W = 600, H = 600;

  // Default quad (roughly centered image area)
  const vertices: Vertex[] = [
    { x: 100, y: 100 },
    { x: 500, y: 50 },
    { x: 500, y: 500 },
    { x: 100, y: 500 },
  ];

  let transType = 0; // 0=affine, 1=bilinear, 2=perspective

  function draw() {
    renderToCanvas({
      demoName: 'image_perspective',
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
  const names = ['Affine Parallelogram', 'Bilinear', 'Perspective'];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'img_persp_trans';
    rb.value = String(i);
    rb.checked = i === transType;
    rb.addEventListener('change', () => { transType = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  const canvasControls: CanvasControl[] = [];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 quad corners to transform the image.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
