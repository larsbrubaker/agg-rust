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
    container, 'Compositing',
    'SVG compositing operations — two shapes blended with selectable comp_op mode.',
  );
  const W = 600, H = 400;
  let compOp = 3, srcAlpha = 0.75, dstAlpha = 1.0;

  function draw() {
    renderToCanvas({ demoName: 'compositing', canvas, width: W, height: H,
      params: [compOp, srcAlpha, dstAlpha], timeDisplay: timeEl });
  }

  const slSrc = addSlider(sidebar, 'Src Alpha', 0, 1, 0.75, 0.01, v => { srcAlpha = v; draw(); });
  const slDst = addSlider(sidebar, 'Dst Alpha', 0, 1, 1, 0.01, v => { dstAlpha = v; draw(); });
  const radios = addRadioGroup(sidebar, 'Comp Op', COMP_OP_NAMES, compOp, i => { compOp = i; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 400, y2: 11, min: 0, max: 1, sidebarEl: slSrc, onChange: v => { srcAlpha = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 400, y2: 26, min: 0, max: 1, sidebarEl: slDst, onChange: v => { dstAlpha = v; draw(); } },
    { type: 'radio', x1: 420, y1: 5, x2: 590, y2: 340, numItems: COMP_OP_NAMES.length, sidebarEls: radios, onChange: i => { compOp = i; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return () => cleanupCC();
}
