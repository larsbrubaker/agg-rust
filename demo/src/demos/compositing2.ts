import { createDemoLayout, addRadioGroup, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

const COMP_OP_NAMES = [
  'clear', 'src', 'dst', 'src-over', 'dst-over', 'src-in', 'dst-in',
  'src-out', 'dst-out', 'src-atop', 'dst-atop', 'xor', 'plus',
  'multiply', 'screen', 'overlay', 'darken', 'lighten',
  'color-dodge', 'color-burn', 'hard-light', 'soft-light', 'difference', 'exclusion',
];

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Compositing 2',
    'Multiple overlapping circles blended with selected SVG compositing mode.',
  );
  const W = 600, H = 400;
  let compOp = 3, srcAlpha = 1.0, dstAlpha = 1.0;

  function draw() {
    renderToCanvas({ demoName: 'compositing2', canvas, width: W, height: H,
      params: [compOp, srcAlpha, dstAlpha], timeDisplay: timeEl });
  }

  const slDst = addSlider(sidebar, 'Dst Alpha', 0, 1, 1, 0.01, v => { dstAlpha = v; draw(); });
  const slSrc = addSlider(sidebar, 'Src Alpha', 0, 1, 1, 0.01, v => { srcAlpha = v; draw(); });
  const displayCompOps = [...COMP_OP_NAMES].reverse();
  const initialDisplayIndex = COMP_OP_NAMES.length - 1 - compOp;
  const radiosDisplay = addRadioGroup(sidebar, 'Comp Op', displayCompOps, initialDisplayIndex, i => {
    compOp = COMP_OP_NAMES.length - 1 - i;
    draw();
  });
  // Keep canvas control sync in AGG logical comp-op index order.
  const radiosByCompOp: HTMLInputElement[] = new Array(COMP_OP_NAMES.length);
  for (let displayIdx = 0; displayIdx < radiosDisplay.length; displayIdx++) {
    const compOpIdx = COMP_OP_NAMES.length - 1 - displayIdx;
    radiosByCompOp[compOpIdx] = radiosDisplay[displayIdx];
  }

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 400, y2: 11, min: 0, max: 1, sidebarEl: slDst, onChange: v => { dstAlpha = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 400, y2: 26, min: 0, max: 1, sidebarEl: slSrc, onChange: v => { srcAlpha = v; draw(); } },
    { type: 'radio', x1: 420, y1: 5, x2: 590, y2: 340, numItems: COMP_OP_NAMES.length, sidebarEls: radiosByCompOp, onChange: i => { compOp = i; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  draw();
  return () => cleanupCC();
}
