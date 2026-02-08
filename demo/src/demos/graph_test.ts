import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Graph Test',
    'Random graph with 200 nodes and 100 edges — matching C++ graph_test.cpp.',
  );

  const W = 700, H = 530;
  let edgeType = 0;
  let strokeWidth = 2.0;
  let drawNodes = true;
  let drawEdges = true;
  let draft = false;
  let translucent = false;

  function draw() {
    renderToCanvas({
      demoName: 'graph_test',
      canvas, width: W, height: H,
      params: [edgeType, strokeWidth, drawNodes ? 1 : 0, drawEdges ? 1 : 0, draft ? 1 : 0, translucent ? 1 : 0],
      timeDisplay: timeEl,
    });
  }

  const radioEls = addRadioGroup(sidebar, 'Edge Type', ['Solid lines', 'Bezier curves', 'Dashed curves', 'Polygons AA', 'Polygons Bin'], 0, v => { edgeType = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0, 5, 2, 0.1, v => { strokeWidth = v; draw(); });
  const cbNodes = addCheckbox(sidebar, 'Draw Nodes', true, v => { drawNodes = v; draw(); });
  const cbEdges = addCheckbox(sidebar, 'Draw Edges', true, v => { drawEdges = v; draw(); });
  const cbDraft = addCheckbox(sidebar, 'Draft Mode', false, v => { draft = v; draw(); });
  const cbTranslucent = addCheckbox(sidebar, 'Translucent Mode', false, v => { translucent = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 5, y1: 35, x2: 110, y2: 110, numItems: 5, sidebarEls: radioEls, onChange: v => { edgeType = v; draw(); } },
    { type: 'slider', x1: 190, y1: 8, x2: 390, y2: 15, min: 0, max: 5, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'checkbox', x1: 398, y1: 21, x2: 485, y2: 34, sidebarEl: cbNodes, onChange: v => { drawNodes = v; draw(); } },
    { type: 'checkbox', x1: 488, y1: 21, x2: 575, y2: 34, sidebarEl: cbEdges, onChange: v => { drawEdges = v; draw(); } },
    { type: 'checkbox', x1: 488, y1: 6, x2: 575, y2: 19, sidebarEl: cbDraft, onChange: v => { draft = v; draw(); } },
    { type: 'checkbox', x1: 190, y1: 21, x2: 395, y2: 34, sidebarEl: cbTranslucent, onChange: v => { translucent = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
