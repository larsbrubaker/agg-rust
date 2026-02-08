import { createDemoLayout, addSlider, addCheckbox, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Filters',
    'Iterative image rotation showing filter quality degradation — matching C++ image_filters.cpp.',
  );

  // Image is 320x300, canvas adds space for controls
  const W = 430, H = 340;
  let filterIdx = 1;
  let stepDeg = 5.0;
  let normalize = true;
  let radius = 4.0;
  let numSteps = 0;
  let running = false;
  let animId = 0;

  function draw() {
    renderToCanvas({
      demoName: 'image_filters',
      canvas, width: W, height: H,
      params: [filterIdx, stepDeg, normalize ? 1 : 0, radius, numSteps],
      timeDisplay: timeEl,
    });
  }

  // Filter selection
  const filterNames = [
    'simple (NN)', 'bilinear', 'bicubic', 'spline16', 'spline36',
    'hanning', 'hamming', 'hermite', 'kaiser', 'quadric', 'catrom',
    'gaussian', 'bessel', 'mitchell', 'sinc', 'lanczos', 'blackman',
  ];
  addRadioGroup(sidebar, 'Filter', filterNames, 1, v => {
    filterIdx = v;
    numSteps = 0;
    draw();
  });

  const slStep = addSlider(sidebar, 'Step', 1, 10, 5, 0.01, v => { stepDeg = v; draw(); });
  const slRadius = addSlider(sidebar, 'Filter Radius', 2, 8, 4, 0.001, v => { radius = v; draw(); });
  const cbNorm = addCheckbox(sidebar, 'Normalize Filter', true, v => { normalize = v; draw(); });

  // Buttons
  function addButton(parent: HTMLElement, label: string, onClick: () => void) {
    const btn = document.createElement('button');
    btn.textContent = label;
    btn.style.cssText = 'display:block;margin:4px 0;padding:4px 12px;cursor:pointer;font-size:12px;';
    btn.addEventListener('click', onClick);
    parent.appendChild(btn);
    return btn;
  }

  addButton(sidebar, 'Single Step', () => {
    numSteps++;
    draw();
  });

  const btnRun = addButton(sidebar, 'RUN Test!', () => {
    if (running) return;
    running = true;
    btnRun.textContent = 'Running...';
    const maxSteps = Math.ceil(360 / stepDeg);
    function step() {
      if (numSteps >= maxSteps || !running) {
        running = false;
        btnRun.textContent = 'RUN Test!';
        return;
      }
      numSteps++;
      draw();
      animId = requestAnimationFrame(step);
    }
    step();
  });

  addButton(sidebar, 'Refresh', () => {
    running = false;
    numSteps = 0;
    draw();
  });

  // Canvas controls
  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 115, y1: 5, x2: W - 15, y2: 11, min: 1, max: 10, sidebarEl: slStep, onChange: v => { stepDeg = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Select a filter, then click RUN to see quality degradation over a full 360° rotation.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    running = false;
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
