import { createDemoLayout, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Perspective',
    'Lion with bilinear/perspective quad transform — matching C++ perspective.cpp.',
  );

  const W = 600, H = 500;

  // Default quad corners (centered lion bounding box ~0,0 to 240,380)
  const ox = (W - 240) / 2;
  const oy = (H - 380) / 2;
  const vertices: Vertex[] = [
    { x: ox, y: oy },           // top-left
    { x: ox + 240, y: oy },     // top-right
    { x: ox + 240, y: oy + 380 }, // bottom-right
    { x: ox, y: oy + 380 },     // bottom-left
  ];

  let transType = 0;

  function draw() {
    renderToCanvas({
      demoName: 'perspective',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        vertices[3].x, vertices[3].y,
        transType,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw,
  });

  addRadioGroup(sidebar, 'Transform', ['Bilinear', 'Perspective'], 0, v => { transType = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 quad corners to warp the lion.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
