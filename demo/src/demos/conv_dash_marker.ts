import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Dash Marker',
    'Dashed stroke with cap styles — matching C++ conv_dash_marker.cpp layout.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let capType = 0;
  let strokeWidth = 3.0;
  let smooth = 1.0;
  let closePoly = false;
  let evenOdd = false;

  function draw() {
    renderToCanvas({
      demoName: 'conv_dash_marker',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        capType, strokeWidth, closePoly ? 1 : 0, evenOdd ? 1 : 0, smooth,
      ],
      timeDisplay: timeEl,
    });
  }

  function pointInTriangle(px: number, py: number, a: Vertex, b: Vertex, c: Vertex): boolean {
    const v0x = c.x - a.x;
    const v0y = c.y - a.y;
    const v1x = b.x - a.x;
    const v1y = b.y - a.y;
    const v2x = px - a.x;
    const v2y = py - a.y;
    const dot00 = v0x * v0x + v0y * v0y;
    const dot01 = v0x * v1x + v0y * v1y;
    const dot02 = v0x * v2x + v0y * v2y;
    const dot11 = v1x * v1x + v1y * v1y;
    const dot12 = v1x * v2x + v1y * v2y;
    const invDenom = 1 / (dot00 * dot11 - dot01 * dot01);
    const u = (dot11 * dot02 - dot01 * dot12) * invDenom;
    const v = (dot00 * dot12 - dot01 * dot02) * invDenom;
    return u >= 0 && v >= 0 && (u + v) <= 1;
  }

  const cleanupDrag = setupVertexDrag({
    canvas, vertices, threshold: 10, onDrag: draw,
    dragAll: true,
    dragAllHitTest: (x, y, vs) => pointInTriangle(x, y, vs[0], vs[1], vs[2]),
  });

  const radioEls = addRadioGroup(sidebar, 'Cap Style', ['Butt Cap', 'Square Cap', 'Round Cap'], 0, v => { capType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0, 10, 3, 0.01, v => { strokeWidth = v; draw(); });
  const slSmooth = addSlider(sidebar, 'Smooth', 0, 2, 1, 0.01, v => { smooth = v; draw(); });
  const cbClose = addCheckbox(sidebar, 'Close Polygons', false, v => { closePoly = v; draw(); });
  const cbEvenOdd = addCheckbox(sidebar, 'Even-Odd Fill', false, v => { evenOdd = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: v => { capType = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 10, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'slider', x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 2, sidebarEl: slSmooth, onChange: v => { smooth = v; draw(); } },
    { type: 'checkbox', x1: 140, y1: 25, x2: 290, y2: 40, sidebarEl: cbClose, onChange: v => { closePoly = v > 0.5; draw(); } },
    { type: 'checkbox', x1: 290, y1: 25, x2: 490, y2: 40, sidebarEl: cbEvenOdd, onChange: v => { evenOdd = v > 0.5; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices. Dashed strokes with arrowhead markers.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
