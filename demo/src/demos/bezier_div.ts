import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Bezier Div',
    'Cubic Bezier curve with draggable control points — matching C++ bezier_div.cpp.',
  );

  const W = 600, H = 500;

  const vertices: Vertex[] = [
    { x: 170, y: 424 },
    { x: 13, y: 87 },
    { x: 488, y: 423 },
    { x: 26, y: 333 },
  ];

  let strokeWidth = 50.0;
  let showPoints = true;
  let showOutline = true;

  function draw() {
    renderToCanvas({
      demoName: 'bezier_div',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        vertices[3].x, vertices[3].y,
        strokeWidth,
        showPoints ? 1 : 0,
        showOutline ? 1 : 0,
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

  const slWidth = addSlider(sidebar, 'Width', -50, 100, 50.0, 1, v => { strokeWidth = v; draw(); });
  const cbPts = addCheckbox(sidebar, 'Show Points', true, v => { showPoints = v; draw(); });
  const cbOutline = addCheckbox(sidebar, 'Show Outline', true, v => { showOutline = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 245, y1: 5, x2: 495, y2: 12, min: -50, max: 100, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'checkbox', x1: 250, y1: 15, x2: 400, y2: 30, sidebarEl: cbPts, onChange: v => { showPoints = v; draw(); } },
    { type: 'checkbox', x1: 250, y1: 30, x2: 450, y2: 45, sidebarEl: cbOutline, onChange: v => { showOutline = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 control points. Red = endpoints, green = handles.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
