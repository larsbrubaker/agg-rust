import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Dash',
    'Dashed stroke with cap styles — based on C++ conv_dash_marker.cpp.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let capType = 0;
  let strokeWidth = 3;
  let closePoly = false;
  let evenOdd = false;

  function draw() {
    renderToCanvas({
      demoName: 'conv_dash',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        capType, strokeWidth,
        closePoly ? 1 : 0, evenOdd ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw,
  });

  addRadioGroup(sidebar, 'Cap', ['Butt Cap', 'Square Cap', 'Round Cap'], 0, v => { capType = v; draw(); });
  addSlider(sidebar, 'Width', 0.5, 10, 3, 0.5, v => { strokeWidth = v; draw(); });
  addCheckbox(sidebar, 'Close Polygons', false, v => { closePoly = v; draw(); });
  addCheckbox(sidebar, 'Even-Odd Fill', false, v => { evenOdd = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
