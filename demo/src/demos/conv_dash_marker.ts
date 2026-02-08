import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Dash Marker',
    'Dashed stroke with cap styles — matching C++ conv_dash_marker.cpp layout.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let capType = 0;
  let strokeWidth = 3.0;
  let closePoly = false;
  let evenOdd = false;

  function draw() {
    renderToCanvas({
      demoName: 'conv_dash_marker',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        capType, strokeWidth, closePoly ? 1 : 0, evenOdd ? 1 : 0,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas, vertices, threshold: 10, onDrag: draw,
  });

  const radioEls = addRadioGroup(sidebar, 'Cap Style', ['Butt Cap', 'Square Cap', 'Round Cap'], 0, v => { capType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0, 10, 3, 0.01, v => { strokeWidth = v; draw(); });
  const cbClose = addCheckbox(sidebar, 'Close Polygons', false, v => { closePoly = v; draw(); });
  const cbEvenOdd = addCheckbox(sidebar, 'Even-Odd Fill', false, v => { evenOdd = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: v => { capType = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 290, y2: 22, min: 0, max: 10, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'checkbox', x1: 140, y1: 34, x2: 290, y2: 47, sidebarEl: cbClose, onChange: v => { closePoly = v > 0.5; draw(); } },
    { type: 'checkbox', x1: 300, y1: 34, x2: 490, y2: 47, sidebarEl: cbEvenOdd, onChange: v => { evenOdd = v > 0.5; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices. Dashed strokes with arrowhead markers (simplified).';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
