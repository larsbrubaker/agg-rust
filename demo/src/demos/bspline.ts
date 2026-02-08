import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'B-Spline',
    'B-spline curve through 6 draggable control points — matching C++ bspline.cpp.',
  );

  const W = 600, H = 600;

  const vertices: Vertex[] = [
    { x: 50, y: 50 },
    { x: 150, y: 550 },
    { x: 250, y: 50 },
    { x: 350, y: 550 },
    { x: 450, y: 50 },
    { x: 550, y: 550 },
  ];

  let numPoints = 20;
  let close = false;

  function draw() {
    renderToCanvas({
      demoName: 'bspline',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        numPoints,
        close ? 1 : 0,
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

  const slPoints = addSlider(sidebar, 'Num Points', 1, 40, 20, 1, v => { numPoints = v; draw(); });
  const cbClose = addCheckbox(sidebar, 'Close', false, v => { close = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 595, y2: 15, min: 1, max: 40, sidebarEl: slPoints, onChange: v => { numPoints = v; draw(); } },
    { type: 'checkbox', x1: 5, y1: 20, x2: 100, y2: 32, sidebarEl: cbClose, onChange: v => { close = v > 0.5; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 6 control points. Red line = B-spline curve, gray = control polygon.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
