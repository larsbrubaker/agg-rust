import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Stroke',
    'Stroke joins, caps, and dashed overlay — matching C++ conv_stroke.cpp.',
  );

  const W = 600, H = 400;

  // Default vertex positions matching C++ conv_stroke.cpp (offset +100 from gouraud)
  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let joinType = 2;   // Round
  let capType = 2;    // Round
  let strokeWidth = 20.0;
  let miterLimit = 4.0;

  function draw() {
    renderToCanvas({
      demoName: 'conv_stroke',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        joinType, capType, strokeWidth, miterLimit,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanup = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    dragAll: true,
    onDrag: draw,
  });

  addRadioGroup(sidebar, 'Line Join', ['Miter', 'Miter Revert', 'Round', 'Bevel'], 2,
    v => { joinType = v; draw(); });
  addRadioGroup(sidebar, 'Line Cap', ['Butt', 'Square', 'Round'], 2,
    v => { capType = v; draw(); });
  addSlider(sidebar, 'Width', 3.0, 40.0, 20.0, 0.5, v => { strokeWidth = v; draw(); });
  addSlider(sidebar, 'Miter Limit', 1.0, 10.0, 4.0, 0.1, v => { miterLimit = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 3 vertices or click inside to move all.';
  sidebar.appendChild(hint);

  draw();
  return cleanup;
}
