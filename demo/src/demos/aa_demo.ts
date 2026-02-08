import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'AA Demo',
    'Anti-aliasing visualization — enlarged pixel view of a triangle.',
  );

  const W = 600, H = 500;

  const vertices: Vertex[] = [
    { x: 100, y: 48 },
    { x: 369, y: 170 },
    { x: 143, y: 310 },
  ];

  let pixelSize = 32;

  function draw() {
    renderToCanvas({
      demoName: 'aa_demo',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        pixelSize,
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

  addSlider(sidebar, 'Pixel Size', 8, 64, 32, 4, v => { pixelSize = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices. Each square shows AA coverage.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
