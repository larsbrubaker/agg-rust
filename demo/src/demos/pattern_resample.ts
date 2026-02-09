import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Pattern Resample',
    'Perspective-transformed procedural image with gamma control — matching C++ pattern_resample.cpp.',
  );

  const W = 600, H = 600;

  const vertices: Vertex[] = [
    { x: W * 0.2, y: H * 0.2 },
    { x: W * 0.8, y: H * 0.15 },
    { x: W * 0.85, y: H * 0.8 },
    { x: W * 0.15, y: H * 0.85 },
  ];

  let gamma = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'pattern_resample',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        gamma,
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

  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 3.0, 1.0, 0.01, v => { gamma = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 595, y2: 12, min: 0.5, max: 3.0, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 quad corners to transform. Adjust gamma.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
