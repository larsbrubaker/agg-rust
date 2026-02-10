import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Text Along Curve 1',
    'TrueType text warped along a B-spline curve using trans_single_path — matching C++ trans_curve1_ft.cpp.',
  );

  const W = 600, H = 600;

  // Initial control points matching C++ on_init()
  const initPts = [
    { x: 50, y: 50 },
    { x: 170, y: 130 },
    { x: 230, y: 270 },
    { x: 370, y: 330 },
    { x: 430, y: 470 },
    { x: 550, y: 550 },
  ];

  const vertices: Vertex[] = initPts.map(p => ({ ...p }));

  let numPoints = 200;
  let preserveXScale = true;
  let fixedLength = true;
  let closePath = false;
  let animating = false;
  let animId = 0;

  // Animation velocities for each control point (matching C++ m_dx/m_dy)
  const dx: number[] = [0, 0, 0, 0, 0, 0];
  const dy: number[] = [0, 0, 0, 0, 0, 0];

  function draw() {
    renderToCanvas({
      demoName: 'trans_curve1',
      canvas, width: W, height: H,
      params: [
        numPoints,
        ...vertices.flatMap(v => [v.x, v.y]),
        preserveXScale ? 1 : 0,
        fixedLength ? 1 : 0,
        closePath ? 1 : 0,
        animating ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  // Animation logic matching C++ move_point() and on_idle()
  function movePoint(i: number) {
    if (vertices[i].x < 0) { vertices[i].x = 0; dx[i] = -dx[i]; }
    if (vertices[i].x > W) { vertices[i].x = W; dx[i] = -dx[i]; }
    if (vertices[i].y < 0) { vertices[i].y = 0; dy[i] = -dy[i]; }
    if (vertices[i].y > H) { vertices[i].y = H; dy[i] = -dy[i]; }
    vertices[i].x += dx[i];
    vertices[i].y += dy[i];
  }

  function animateFrame() {
    for (let i = 0; i < 6; i++) {
      movePoint(i);
    }
    draw();
    if (animating) animId = requestAnimationFrame(animateFrame);
  }

  function startAnimation(v: boolean) {
    animating = v;
    if (v) {
      // Reset to initial positions and randomize velocities
      // Matching C++ on_ctrl_change(): on_init() then random dx/dy in [-5, +5]
      for (let i = 0; i < 6; i++) {
        vertices[i].x = initPts[i].x;
        vertices[i].y = initPts[i].y;
        dx[i] = (Math.random() * 1000 - 500) * 0.01;
        dy[i] = (Math.random() * 1000 - 500) * 0.01;
      }
      animId = requestAnimationFrame(animateFrame);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw,
  });

  // Sidebar controls
  const slPoints = addSlider(sidebar, 'Num Points', 10, 400, 200, 10, v => { numPoints = v; draw(); });
  const cbClose = addCheckbox(sidebar, 'Close', closePath, v => { closePath = v; draw(); });
  const cbPreserve = addCheckbox(sidebar, 'Preserve X scale', preserveXScale, v => { preserveXScale = v; draw(); });
  const cbFixed = addCheckbox(sidebar, 'Fixed Length', fixedLength, v => { fixedLength = v; draw(); });
  const cbAnimate = addCheckbox(sidebar, 'Animate', false, v => startAnimation(v));

  // Canvas controls — match AGG-rendered control positions for click interaction
  // C++ layout:
  //   m_num_points       (5, 5, 340, 12)      — slider
  //   m_close            (350, 5)              — checkbox
  //   m_preserve_x_scale (460, 5)              — checkbox
  //   m_fixed_len        (350, 25)             — checkbox
  //   m_animate          (460, 25)             — checkbox
  // CboxCtrl checkbox area: 13.5px tall, label extends ~100px right
  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 340, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: v => { numPoints = v; draw(); } },
    { type: 'checkbox', x1: 350, y1: 5, x2: 455, y2: 19, sidebarEl: cbClose, onChange: v => { closePath = v; draw(); } },
    { type: 'checkbox', x1: 460, y1: 5, x2: 595, y2: 19, sidebarEl: cbPreserve, onChange: v => { preserveXScale = v; draw(); } },
    { type: 'checkbox', x1: 350, y1: 25, x2: 455, y2: 39, sidebarEl: cbFixed, onChange: v => { fixedLength = v; draw(); } },
    { type: 'checkbox', x1: 460, y1: 25, x2: 560, y2: 39, sidebarEl: cbAnimate, onChange: v => startAnimation(v) },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 6 control points to reshape the curve. Toggle Animate for bouncing points.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    animating = false;
    cancelAnimationFrame(animId);
    cleanupDrag();
    cleanupCC();
  };
}
