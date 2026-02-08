import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Bezier Div',
    'Cubic Bezier curve with draggable control points — matching C++ bezier_div.cpp.',
  );

  const W = 600, H = 500;

  // Default control points from C++ bezier_div.cpp
  const vertices: Vertex[] = [
    { x: 170, y: 424 },  // P1 (start)
    { x: 13, y: 87 },    // P2 (control 1)
    { x: 488, y: 423 },  // P3 (control 2)
    { x: 26, y: 333 },   // P4 (end)
  ];

  let strokeWidth = 50.0;
  let showPoints = true;
  let showOutline = true;

  function draw() {
    renderToCanvas({
      demoName: 'bezier_div',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        vertices[3].x, vertices[3].y,
        strokeWidth,
        showPoints ? 1 : 0,
        showOutline ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw,
  });

  addSlider(sidebar, 'Width', -50, 100, 50.0, 1, v => { strokeWidth = v; draw(); });
  addCheckbox(sidebar, 'Show Points', true, v => { showPoints = v; draw(); });
  addCheckbox(sidebar, 'Show Outline', true, v => { showOutline = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 control points. Red = endpoints, green = handles.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
