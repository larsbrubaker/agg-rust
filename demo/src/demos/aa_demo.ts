import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

// Matches agg::point_in_triangle used by aa_demo.cpp's on_mouse_button_down:
// clicking inside the triangle grabs the whole shape (m_idx == 3).
function pointInTriangle(
  ax: number, ay: number,
  bx: number, by: number,
  cx: number, cy: number,
  px: number, py: number,
): boolean {
  const sign = (x1: number, y1: number, x2: number, y2: number, x3: number, y3: number) =>
    (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3);
  const d1 = sign(px, py, ax, ay, bx, by);
  const d2 = sign(px, py, bx, by, cx, cy);
  const d3 = sign(px, py, cx, cy, ax, ay);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'AA Demo',
    'Anti-aliasing visualization — enlarged pixel view of a triangle.',
  );

  const W = 600, H = 400;

  const vertices: Vertex[] = [
    { x: 57, y: 100 },
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
    threshold: 10,
    // C++ aa_demo.cpp also lets you drag the whole triangle by clicking inside it
    // (point_in_triangle -> m_idx = 3). Mirror that here.
    dragAll: true,
    dragAllHitTest: (x, y, verts) => pointInTriangle(
      verts[0].x, verts[0].y,
      verts[1].x, verts[1].y,
      verts[2].x, verts[2].y,
      x, y,
    ),
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
