import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Stroke',
    'Stroke joins, caps, and dashed overlay — matching C++ conv_stroke.cpp.',
  );

  const W = 600, H = 400;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let joinType = 2;
  let capType = 2;
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

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    dragAll: true,
    onDrag: draw,
  });

  const joinEls = addRadioGroup(sidebar, 'Line Join', ['Miter', 'Miter Revert', 'Round', 'Bevel'], 2,
    v => { joinType = v; draw(); });
  const capEls = addRadioGroup(sidebar, 'Line Cap', ['Butt', 'Square', 'Round'], 2,
    v => { capType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 3.0, 40.0, 20.0, 0.5, v => { strokeWidth = v; draw(); });
  const slMiter = addSlider(sidebar, 'Miter Limit', 1.0, 10.0, 4.0, 0.1, v => { miterLimit = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 133, y2: 80, numItems: 4, sidebarEls: joinEls, onChange: v => { joinType = v; draw(); } },
    { type: 'radio', x1: 10, y1: 90, x2: 133, y2: 160, numItems: 3, sidebarEls: capEls, onChange: v => { capType = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 490, y2: 22, min: 3, max: 40, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'slider', x1: 140, y1: 34, x2: 490, y2: 42, min: 1, max: 10, sidebarEl: slMiter, onChange: v => { miterLimit = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 3 vertices or click inside to move all.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
