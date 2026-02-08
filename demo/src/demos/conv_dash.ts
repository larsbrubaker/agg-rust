import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Conv Dash',
    'Dashed stroke with cap styles — based on C++ conv_dash_marker.cpp.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 },
  ];

  let capType = 0;
  let strokeWidth = 3;
  let closePoly = false;
  let evenOdd = false;
  let smooth = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'conv_dash',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        capType, strokeWidth,
        closePoly ? 1 : 0, evenOdd ? 1 : 0, smooth,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw,
  });

  const radioEls = addRadioGroup(sidebar, 'Cap', ['Butt Cap', 'Square Cap', 'Round Cap'], 0, v => { capType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0.5, 10, 3, 0.5, v => { strokeWidth = v; draw(); });
  const slSmooth = addSlider(sidebar, 'Smooth', 0, 2, 1, 0.1, v => { smooth = v; draw(); });
  const cbClose = addCheckbox(sidebar, 'Close Polygons', false, v => { closePoly = v; draw(); });
  const cbEO = addCheckbox(sidebar, 'Even-Odd Fill', false, v => { evenOdd = v; draw(); });

  // Canvas control interaction — positions match WASM render
  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: v => { capType = v; draw(); } },
    { type: 'slider', x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 10, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'slider', x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 2, sidebarEl: slSmooth, onChange: v => { smooth = v; draw(); } },
    { type: 'checkbox', x1: 140, y1: 25, x2: 290, y2: 40, sidebarEl: cbClose, onChange: v => { closePoly = v; draw(); } },
    { type: 'checkbox', x1: 300, y1: 25, x2: 450, y2: 40, sidebarEl: cbEO, onChange: v => { evenOdd = v; draw(); } },
  ];
  const cleanupControls = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag triangle vertices. Click canvas controls to interact.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupControls(); };
}
