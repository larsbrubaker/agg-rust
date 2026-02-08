import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Line Thickness',
    'Lines at varying widths — matching C++ line_thickness.cpp.',
  );

  const W = 640, H = 480;

  const vertices: Vertex[] = [
    { x: W * 0.05, y: H * 0.5 },
    { x: W * 0.95, y: H * 0.5 },
  ];

  let thickness = 1.0;
  let blur = 1.5;
  let monochrome = true;
  let invert = false;

  function draw() {
    renderToCanvas({
      demoName: 'line_thickness',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y, vertices[1].x, vertices[1].y,
        thickness, blur, monochrome ? 1 : 0, invert ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    onDrag: draw,
  });

  const slThick = addSlider(sidebar, 'Line thickness', 0, 5, 1, 0.1, v => { thickness = v; draw(); });
  const slBlur = addSlider(sidebar, 'Blur radius', 0, 2, 1.5, 0.1, v => { blur = v; draw(); });
  const cbMono = addCheckbox(sidebar, 'Monochrome', true, v => { monochrome = v; draw(); });
  const cbInvert = addCheckbox(sidebar, 'Invert', false, v => { invert = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 10, y1: 10, x2: 630, y2: 19, min: 0, max: 5, sidebarEl: slThick, onChange: v => { thickness = v; draw(); } },
    { type: 'slider', x1: 10, y1: 30, x2: 630, y2: 39, min: 0, max: 2, sidebarEl: slBlur, onChange: v => { blur = v; draw(); } },
    { type: 'checkbox', x1: 10, y1: 45, x2: 200, y2: 60, sidebarEl: cbMono, onChange: v => { monochrome = v; draw(); } },
    { type: 'checkbox', x1: 10, y1: 65, x2: 200, y2: 80, sidebarEl: cbInvert, onChange: v => { invert = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag endpoints to tilt lines.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
