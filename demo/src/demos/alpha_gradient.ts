import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Gradient',
    'Gradient with alpha curve control over a random ellipse background — matching C++ alpha_gradient.cpp.',
  );

  const W = 512, H = 400;

  // Triangle vertices for gradient center/direction
  const vertices: Vertex[] = [
    { x: 257, y: 60 },
    { x: 369, y: 170 },
    { x: 143, y: 310 },
  ];

  // 6 alpha curve control values (0..1)
  const alphaValues = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

  function draw() {
    renderToCanvas({
      demoName: 'alpha_gradient',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        ...alphaValues,
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

  // Alpha curve sliders
  const controls: CanvasControl[] = [];
  for (let i = 0; i < 6; i++) {
    controls.push({
      type: 'slider',
      label: `Alpha ${i}`,
      min: 0, max: 1, step: 0.01,
      initial: alphaValues[i],
      onChange(v) { alphaValues[i] = v; draw(); },
    });
  }
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 3 triangle vertices. Adjust alpha curve with sliders.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
