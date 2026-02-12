import { addRadioGroup, createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Perspective',
    'Image transformed through affine/bilinear/perspective quad — matching C++ image_perspective.cpp.',
  );

  const W = 600, H = 600;

  // C++ defaults from image_perspective.cpp on_init().
  const vertices: Vertex[] = [
    { x: 100, y: 100 },
    { x: W - 100, y: 100 },
    { x: W - 100, y: H - 100 },
    { x: 100, y: H - 100 },
  ];

  let transType = 0; // 0=affine, 1=bilinear, 2=perspective

  function draw() {
    renderToCanvas({
      demoName: 'image_perspective',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        transType,
      ],
      timeDisplay: timeEl,
    });
  }

  function pointInPolygon(x: number, y: number, pts: Vertex[]): boolean {
    let inside = false;
    for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
      const xi = pts[i].x;
      const yi = pts[i].y;
      const xj = pts[j].x;
      const yj = pts[j].y;
      const intersects = ((yi > y) !== (yj > y)) &&
        (x < ((xj - xi) * (y - yi)) / ((yj - yi) || 1e-12) + xi);
      if (intersects) inside = !inside;
    }
    return inside;
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    dragEdges: true,
    edgeThreshold: 5,
    dragAll: true,
    dragAllHitTest: (x, y, verts) => pointInPolygon(x, y, verts),
    onDrag: draw,
  });

  const names = ['Affine Parallelogram', 'Bilinear', 'Perspective'];
  // rbox first item is at the bottom in AGG, so reverse sidebar display order.
  const namesUi = [...names].reverse();
  const radioInputsUi = addRadioGroup(sidebar, 'Transform Type', namesUi, names.length - 1 - transType, (uiIdx) => {
    transType = names.length - 1 - uiIdx;
    draw();
  });
  const radioInputs: HTMLInputElement[] = new Array(names.length);
  for (let uiIdx = 0; uiIdx < names.length; uiIdx++) {
    const logicalIdx = names.length - 1 - uiIdx;
    radioInputs[logicalIdx] = radioInputsUi[uiIdx];
  }

  const canvasControls: CanvasControl[] = [
    {
      type: 'radio',
      x1: 420,
      y1: 5,
      x2: 590,
      y2: 65,
      numItems: 3,
      sidebarEls: radioInputs,
      onChange(index: number) {
        transType = index;
        draw();
      },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag corners, edges, or inside the quad. Canvas and sidebar transform controls are synchronized.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
