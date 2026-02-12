import { createDemoLayout, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Perspective',
    'Lion with bilinear/perspective quad transform — matching C++ perspective.cpp.',
  );

  const W = 600, H = 600;

  const ox = (W - 240) / 2;
  const oy = (H - 380) / 2;
  const vertices: Vertex[] = [
    { x: ox, y: oy },
    { x: ox + 240, y: oy },
    { x: ox + 240, y: oy + 380 },
    { x: ox, y: oy + 380 },
  ];

  let transType = 0;

  function pointInPolygon(x: number, y: number, verts: Vertex[]): boolean {
    let inside = false;
    for (let i = 0, j = verts.length - 1; i < verts.length; j = i++) {
      const xi = verts[i].x;
      const yi = verts[i].y;
      const xj = verts[j].x;
      const yj = verts[j].y;
      const intersects = ((yi > y) !== (yj > y)) &&
        (x < (xj - xi) * (y - yi) / ((yj - yi) || 1e-12) + xi);
      if (intersects) inside = !inside;
    }
    return inside;
  }

  function draw() {
    renderToCanvas({
      demoName: 'perspective',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        vertices[3].x, vertices[3].y,
        transType,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 5,
    dragEdges: true,
    edgeThreshold: 5,
    dragAll: true,
    dragAllHitTest: pointInPolygon,
    onDrag: draw,
  });

  const radioEls = addRadioGroup(sidebar, 'Transform', ['Bilinear', 'Perspective'], 0, v => { transType = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 420, y1: 5, x2: 550, y2: 55, numItems: 2, sidebarEls: radioEls, onChange: v => { transType = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 quad corners to warp the lion.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
