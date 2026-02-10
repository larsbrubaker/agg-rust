import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Text Along Curve 2',
    'TrueType text warped between two B-spline curves using trans_double_path — matching C++ trans_curve2_ft.cpp.',
  );

  const W = 600, H = 600;

  // Initial control points matching C++ on_init()
  // poly1: offset (+10, -10) from diagonal
  const initPoly1 = [
    { x: 60, y: 40 },
    { x: 180, y: 120 },
    { x: 240, y: 260 },
    { x: 380, y: 320 },
    { x: 440, y: 460 },
    { x: 560, y: 540 },
  ];
  // poly2: offset (-10, +10) from diagonal
  const initPoly2 = [
    { x: 40, y: 60 },
    { x: 160, y: 140 },
    { x: 220, y: 280 },
    { x: 360, y: 340 },
    { x: 420, y: 480 },
    { x: 540, y: 560 },
  ];

  const poly1: Vertex[] = initPoly1.map(p => ({ ...p }));
  const poly2: Vertex[] = initPoly2.map(p => ({ ...p }));

  let numPoints = 200;
  let preserveXScale = true;
  let fixedLength = true;
  let animating = false;
  let animId = 0;

  // Animation velocities for both polygon sets
  const dx1: number[] = [0, 0, 0, 0, 0, 0];
  const dy1: number[] = [0, 0, 0, 0, 0, 0];
  const dx2: number[] = [0, 0, 0, 0, 0, 0];
  const dy2: number[] = [0, 0, 0, 0, 0, 0];

  function draw() {
    renderToCanvas({
      demoName: 'trans_curve2',
      canvas, width: W, height: H,
      params: [
        numPoints,
        ...poly1.flatMap(v => [v.x, v.y]),
        ...poly2.flatMap(v => [v.x, v.y]),
        preserveXScale ? 1 : 0,
        fixedLength ? 1 : 0,
        animating ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  // Animation logic matching C++ move_point() and normalize_point()
  function movePoint(v: Vertex, pdx: number[], pdy: number[], i: number) {
    if (v.x < 0) { v.x = 0; pdx[i] = -pdx[i]; }
    if (v.x > W) { v.x = W; pdx[i] = -pdx[i]; }
    if (v.y < 0) { v.y = 0; pdy[i] = -pdy[i]; }
    if (v.y > H) { v.y = H; pdy[i] = -pdy[i]; }
    v.x += pdx[i];
    v.y += pdy[i];
  }

  // C++ normalize_point: keep poly2 within 28.28 pixels of poly1
  function normalizePoint(i: number) {
    const ddx = poly2[i].x - poly1[i].x;
    const ddy = poly2[i].y - poly1[i].y;
    const d = Math.sqrt(ddx * ddx + ddy * ddy);
    if (d > 28.28) {
      poly2[i].x = poly1[i].x + ddx * 28.28 / d;
      poly2[i].y = poly1[i].y + ddy * 28.28 / d;
    }
  }

  function animateFrame() {
    for (let i = 0; i < 6; i++) {
      movePoint(poly1[i], dx1, dy1, i);
      movePoint(poly2[i], dx2, dy2, i);
      normalizePoint(i);
    }
    draw();
    if (animating) animId = requestAnimationFrame(animateFrame);
  }

  function startAnimation(v: boolean) {
    animating = v;
    if (v) {
      for (let i = 0; i < 6; i++) {
        poly1[i].x = initPoly1[i].x;
        poly1[i].y = initPoly1[i].y;
        poly2[i].x = initPoly2[i].x;
        poly2[i].y = initPoly2[i].y;
        dx1[i] = (Math.random() * 1000 - 500) * 0.01;
        dy1[i] = (Math.random() * 1000 - 500) * 0.01;
        dx2[i] = (Math.random() * 1000 - 500) * 0.01;
        dy2[i] = (Math.random() * 1000 - 500) * 0.01;
      }
      animId = requestAnimationFrame(animateFrame);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }

  // Both polygon sets are draggable
  const allVertices = [...poly1, ...poly2];
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices: allVertices,
    threshold: 10,
    onDrag: draw,
  });

  // Sidebar controls
  const slPoints = addSlider(sidebar, 'Num Points', 10, 400, 200, 10, v => { numPoints = v; draw(); });
  const cbFixed = addCheckbox(sidebar, 'Fixed Length', fixedLength, v => { fixedLength = v; draw(); });
  const cbPreserve = addCheckbox(sidebar, 'Preserve X scale', preserveXScale, v => { preserveXScale = v; draw(); });
  const cbAnimate = addCheckbox(sidebar, 'Animate', false, v => startAnimation(v));

  // Canvas controls matching AGG-rendered positions
  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 340, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: v => { numPoints = v; draw(); } },
    { type: 'checkbox', x1: 350, y1: 5, x2: 460, y2: 19, sidebarEl: cbFixed, onChange: v => { fixedLength = v; draw(); } },
    { type: 'checkbox', x1: 465, y1: 5, x2: 595, y2: 19, sidebarEl: cbPreserve, onChange: v => { preserveXScale = v; draw(); } },
    { type: 'checkbox', x1: 350, y1: 25, x2: 460, y2: 39, sidebarEl: cbAnimate, onChange: v => startAnimation(v) },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 12 control points (6 per curve) to reshape both curves.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    animating = false;
    cancelAnimationFrame(animId);
    cleanupDrag();
    cleanupCC();
  };
}
