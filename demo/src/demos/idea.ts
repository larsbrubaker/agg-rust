import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Idea',
    'Rotating light bulb icon with fill options — matching C++ idea.cpp.',
  );

  const W = 250, H = 280;
  let angle = 0;
  let evenOdd = false;
  let draft = false;
  let roundoff = false;
  let angleDelta = 0.01;
  let rotating = false;
  let animId = 0;

  function draw() {
    renderToCanvas({
      demoName: 'idea',
      canvas, width: W, height: H,
      params: [angle, evenOdd ? 1 : 0, draft ? 1 : 0, roundoff ? 1 : 0, angleDelta, rotating ? 1 : 0],
      timeDisplay: timeEl,
    });
  }

  function animate() {
    angle += angleDelta;
    draw();
    if (rotating) animId = requestAnimationFrame(animate);
  }

  function startStop(v: boolean) {
    rotating = v;
    if (v) {
      animId = requestAnimationFrame(animate);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }

  const cbRotate = addCheckbox(sidebar, 'Rotate', false, v => startStop(v));
  const cbEvenOdd = addCheckbox(sidebar, 'Even-Odd', false, v => { evenOdd = v; draw(); });
  const cbDraft = addCheckbox(sidebar, 'Draft', false, v => { draft = v; draw(); });
  const cbRoundoff = addCheckbox(sidebar, 'Roundoff', false, v => { roundoff = v; draw(); });
  const slStep = addSlider(sidebar, 'Step (degrees)', 0, 0.1, 0.01, 0.001, v => { angleDelta = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'checkbox', x1: 10, y1: 3, x2: 55, y2: 16, sidebarEl: cbRotate, onChange: v => startStop(v) },
    { type: 'checkbox', x1: 60, y1: 3, x2: 125, y2: 16, sidebarEl: cbEvenOdd, onChange: v => { evenOdd = v; draw(); } },
    { type: 'checkbox', x1: 130, y1: 3, x2: 170, y2: 16, sidebarEl: cbDraft, onChange: v => { draft = v; draw(); } },
    { type: 'checkbox', x1: 175, y1: 3, x2: 240, y2: 16, sidebarEl: cbRoundoff, onChange: v => { roundoff = v; draw(); } },
    { type: 'slider', x1: 10, y1: 21, x2: 240, y2: 27, min: 0, max: 0.1, sidebarEl: slStep, onChange: v => { angleDelta = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return () => {
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
