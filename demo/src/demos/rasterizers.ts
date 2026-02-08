import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Rasterizers',
    'Filled and stroked triangle with draggable vertices.',
  );

  const W = 600, H = 400;

  const vertices: Vertex[] = [
    { x: 100, y: 60 },
    { x: 400, y: 80 },
    { x: 250, y: 350 },
  ];

  let gammaVal = 1.0;
  let alpha = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'rasterizers',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        gammaVal, alpha,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    dragAll: true,
    onDrag: draw,
  });

  const slGamma = addSlider(sidebar, 'Gamma', 0.0, 1.0, 1.0, 0.01, v => { gammaVal = v; draw(); });
  const slAlpha = addSlider(sidebar, 'Alpha', 0.0, 1.0, 1.0, 0.01, v => { alpha = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 1, sidebarEl: slGamma, onChange: v => { gammaVal = v; draw(); } },
    { type: 'slider', x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 1, sidebarEl: slAlpha, onChange: v => { alpha = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 3 vertices.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
