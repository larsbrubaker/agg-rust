import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Rounded Rect',
    'Draggable rounded rectangle — matching C++ rounded_rect.cpp.',
  );

  const W = 600, H = 400;

  const vertices: Vertex[] = [
    { x: 100, y: 80 },
    { x: 400, y: 280 },
  ];

  let radius = 20;
  let outlineWidth = 2;

  function draw() {
    renderToCanvas({
      demoName: 'rounded_rect',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        radius, outlineWidth,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    onDrag: draw,
  });

  addSlider(sidebar, 'Radius', 0, 80, 20, 1, v => { radius = v; draw(); });
  addSlider(sidebar, 'Outline Width', 0.5, 10, 2, 0.5, v => { outlineWidth = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the two corner points.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
