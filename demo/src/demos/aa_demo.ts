import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

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

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw,
  });

  const slPixel = addSlider(sidebar, 'Pixel Size', 8, 100, 32, 1, v => { pixelSize = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 80, y1: 10, x2: W - 10, y2: 19, min: 8, max: 100, sidebarEl: slPixel, onChange: v => { pixelSize = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices. Each square shows AA coverage.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
