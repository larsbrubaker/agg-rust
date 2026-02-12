import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

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
    'Conv Stroke',
    'Stroke joins, caps, and dashed overlay — matching C++ conv_stroke.cpp.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let joinType = 2;
  let capType = 2;
  let strokeWidth = 20.0;
  let miterLimit = 4.0;

  function draw() {
    renderToCanvas({
      demoName: 'conv_stroke',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        joinType, capType, strokeWidth, miterLimit,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    dragAll: true,
    dragAllHitTest: (x, y, verts) => pointInTriangle(
      verts[0].x, verts[0].y,
      verts[1].x, verts[1].y,
      verts[2].x, verts[2].y,
      x, y,
    ),
    onDrag: draw,
  });

  const joinEls = addRadioGroup(sidebar, 'Line Join', ['Miter Join', 'Miter Join Revert', 'Round Join', 'Bevel Join'], 2,
    v => { joinType = v; draw(); });
  const capEls = addRadioGroup(sidebar, 'Line Cap', ['Butt Cap', 'Square Cap', 'Round Cap'], 2,
    v => { capType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 3.0, 40.0, 20.0, 0.01, v => { strokeWidth = v; draw(); });
  const slMiter = addSlider(sidebar, 'Miter Limit', 1.0, 10.0, 4.0, 0.01, v => { miterLimit = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 133, y2: 80, numItems: 4, textHeight: 7.5, sidebarEls: joinEls, onChange: v => { joinType = v; draw(); } },
    { type: 'radio', x1: 10, y1: 90, x2: 133, y2: 160, numItems: 3, sidebarEls: capEls, onChange: v => { capType = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 490, y2: 22, min: 3, max: 40, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'slider', x1: 140, y1: 34, x2: 490, y2: 42, min: 1, max: 10, sidebarEl: slMiter, onChange: v => { miterLimit = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  // Match C++ conv_stroke.cpp: arrow keys move only vertices 0 and 1.
  function onKeyDown(e: KeyboardEvent) {
    let dx = 0;
    let dy = 0;
    switch (e.key) {
      case 'ArrowLeft': dx = -0.1; break;
      case 'ArrowRight': dx = 0.1; break;
      case 'ArrowUp': dy = 0.1; break;
      case 'ArrowDown': dy = -0.1; break;
      default: return;
    }
    vertices[0].x += dx;
    vertices[0].y += dy;
    vertices[1].x += dx;
    vertices[1].y += dy;
    draw();
    e.preventDefault();
  }
  window.addEventListener('keydown', onKeyDown);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 3 vertices or click inside to move all.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
    window.removeEventListener('keydown', onKeyDown);
  };
}
