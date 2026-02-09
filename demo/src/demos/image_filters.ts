import { createDemoLayout, addSlider, addCheckbox, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Filters',
    'Iterative image rotation showing filter quality degradation — matching C++ image_filters.cpp.',
  );

  // Image is 320x300, canvas adds space for controls (matching C++ window size)
  const W = 430, H = 340;
  let filterIdx = 1;
  let stepDeg = 5.0;
  let normalize = true;
  let radius = 4.0;
  let numSteps = 0;
  let kpixSec = 0.0;
  let running = false;
  let animId = 0;

  // Timing for Kpix/sec measurement during RUN
  let runStartTime = 0;
  let runTotalPixels = 0;
  const IMG_PIXELS = 320 * 300; // spheres.bmp dimensions

  function draw(incremental = false) {
    renderToCanvas({
      demoName: 'image_filters',
      canvas, width: W, height: H,
      params: [filterIdx, stepDeg, normalize ? 1 : 0, radius, numSteps, kpixSec, incremental ? 1 : 0],
      timeDisplay: timeEl,
    });
  }

  // Filter selection
  const filterNames = [
    'simple (NN)', 'bilinear', 'bicubic', 'spline16', 'spline36',
    'hanning', 'hamming', 'hermite', 'kaiser', 'quadric', 'catrom',
    'gaussian', 'bessel', 'mitchell', 'sinc', 'lanczos', 'blackman',
  ];
  const radioEls = addRadioGroup(sidebar, 'Filter', filterNames, 1, v => {
    filterIdx = v;
    numSteps = 0;
    kpixSec = 0;
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

  function doSingleStep() {
    numSteps++;
    draw(true); // incremental: one transform from cached state
  }

  function doRun() {
    if (running) return;
    running = true;
    btnRun.textContent = 'Running...';
    kpixSec = 0;
    const maxSteps = Math.ceil(360 / stepDeg);
    runStartTime = performance.now();
    runTotalPixels = 0;
    function step() {
      if (numSteps >= maxSteps || !running) {
        running = false;
        btnRun.textContent = 'RUN Test!';
        // Compute final Kpix/sec
        const elapsed = (performance.now() - runStartTime) / 1000.0; // seconds
        if (elapsed > 0 && runTotalPixels > 0) {
          kpixSec = (runTotalPixels / 1000.0) / elapsed;
        }
        draw(); // Final render with Kpix/sec displayed
        return;
      }
      numSteps++;
      runTotalPixels += IMG_PIXELS;
      draw(true); // incremental: one transform from cached state
      animId = requestAnimationFrame(step);
    }
    step();
  }

  function doRefresh() {
    running = false;
    numSteps = 0;
    kpixSec = 0;
    draw();
  }

  addButton(sidebar, 'Single Step', doSingleStep);
  const btnRun = addButton(sidebar, 'RUN Test!', doRun);
  addButton(sidebar, 'Refresh', doRefresh);

  // Canvas controls — make AGG-rendered controls interactable
  const canvasControls: CanvasControl[] = [
    // Step slider at (115, 5, 400, 11)
    {
      type: 'slider', x1: 115, y1: 5, x2: 400, y2: 11,
      min: 1, max: 10, sidebarEl: slStep,
      onChange: v => { stepDeg = v; draw(); },
    },
    // Radius slider at (115, 20, 400, 26)
    {
      type: 'slider', x1: 115, y1: 20, x2: 400, y2: 26,
      min: 2, max: 8, sidebarEl: slRadius,
      onChange: v => { radius = v; draw(); },
    },
    // Filter radio box at (0, 0, 110, 210) with 17 items
    {
      type: 'radio', x1: 0, y1: 0, x2: 110, y2: 210,
      numItems: 17, sidebarEls: radioEls,
      onChange: idx => { filterIdx = idx; numSteps = 0; kpixSec = 0; draw(); },
    },
    // Normalize Filter checkbox at (8, 215) — approx bounds
    {
      type: 'checkbox', x1: 8, y1: 215, x2: 110, y2: 228,
      sidebarEl: cbNorm,
      onChange: v => { normalize = v; draw(); },
    },
    // Single Step checkbox at (8, 230) — acts as button
    {
      type: 'button', x1: 8, y1: 230, x2: 100, y2: 243,
      onClick: doSingleStep,
    },
    // RUN Test! checkbox at (8, 245) — acts as button
    {
      type: 'button', x1: 8, y1: 245, x2: 80, y2: 258,
      onClick: doRun,
    },
    // Refresh checkbox at (8, 265) — acts as button
    {
      type: 'button', x1: 8, y1: 265, x2: 75, y2: 278,
      onClick: doRefresh,
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Select a filter, then click RUN to see quality degradation over a full 360° rotation. Controls on canvas are interactive.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    running = false;
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
