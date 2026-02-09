import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Text Along Curve 1',
    'Text warped along a B-spline curve using trans_single_path — matching C++ trans_curve1_test.cpp.',
  );

  const W = 600, H = 600;

  const vertices: Vertex[] = [
    { x: 100, y: 400 },
    { x: 200, y: 200 },
    { x: 300, y: 500 },
    { x: 400, y: 100 },
    { x: 500, y: 350 },
    { x: 550, y: 300 },
  ];

  let numPoints = 200;

  function draw() {
    renderToCanvas({
      demoName: 'trans_curve1',
      canvas, width: W, height: H,
      params: [
        numPoints,
        ...vertices.flatMap(v => [v.x, v.y]),
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw,
  });

  const slPoints = addSlider(sidebar, 'Num Points', 10, 400, 200, 10, v => { numPoints = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 595, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: v => { numPoints = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 6 control points to reshape the curve.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
