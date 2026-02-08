import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Line Thickness',
    'Lines at varying widths from 0.1 to 5.0 pixels.',
  );

  const W = 600, H = 400;

  const vertices: Vertex[] = [
    { x: W * 0.05, y: H * 0.5 },
    { x: W * 0.95, y: H * 0.5 },
  ];

  function draw() {
    renderToCanvas({
      demoName: 'line_thickness',
      canvas, width: W, height: H,
      params: [vertices[0].x, vertices[0].y, vertices[1].x, vertices[1].y],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    onDrag: draw,
  });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag endpoints to tilt lines.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
