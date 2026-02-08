import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gouraud Shading',
    '6 sub-triangles with draggable vertices — matching C++ gouraud.cpp.',
  );

  const W = 400, H = 320;

  const vertices: Vertex[] = [
    { x: 57, y: 60 },
    { x: 369, y: 170 },
    { x: 143, y: 310 },
  ];

  let dilation = 0.175;
  let gamma = 0.809;
  let alpha = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gouraud',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        dilation, gamma, alpha,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    dragAll: true,
    onDrag: draw,
  });

  const slDilation = addSlider(sidebar, 'Dilation', 0.0, 1.0, 0.175, 0.025, v => { dilation = v; draw(); });
  const slGamma = addSlider(sidebar, 'Gamma', 0.0, 3.0, 0.809, 0.01, v => { gamma = v; draw(); });
  const slAlpha = addSlider(sidebar, 'Alpha', 0.0, 1.0, 1.0, 0.01, v => { alpha = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 395, y2: 11, min: 0, max: 1, sidebarEl: slDilation, onChange: v => { dilation = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 395, y2: 26, min: 0, max: 3, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: 395, y2: 41, min: 0, max: 1, sidebarEl: slAlpha, onChange: v => { alpha = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag vertices or click inside triangle to move all.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
