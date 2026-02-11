import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Rasterizer Compound',
    'Compound rasterizer with layer order control — matching C++ rasterizer_compound.cpp.',
  );
  const W = 440;
  const H = 330;
  let strokeWidth = 10.0;
  let alpha1 = 1.0;
  let alpha2 = 1.0;
  let alpha3 = 1.0;
  let alpha4 = 1.0;
  let invertOrder = 0;

  type SliderState = {
    input: HTMLInputElement;
    valueEl: HTMLElement;
    min: number;
    max: number;
    decimals: number;
    get: () => number;
    set: (v: number) => void;
  };

  let suppressSidebarEvents = false;
  let activeCanvasSlider: number = -1;

  function clamp(v: number, lo: number, hi: number): number {
    return Math.max(lo, Math.min(hi, v));
  }

  function makeSlider(
    labelPrefix: string,
    min: number,
    max: number,
    step: number,
    decimals: number,
    get: () => number,
    set: (v: number) => void,
  ): SliderState {
    const group = document.createElement('div');
    group.className = 'control-group';

    const label = document.createElement('label');
    label.className = 'control-label';
    label.textContent = labelPrefix;
    group.appendChild(label);

    const input = document.createElement('input');
    input.className = 'control-slider';
    input.type = 'range';
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(get());
    group.appendChild(input);

    const valueEl = document.createElement('span');
    valueEl.className = 'control-value';
    group.appendChild(valueEl);
    sidebar.appendChild(group);

    const state: SliderState = { input, valueEl, min, max, decimals, get, set };
    const applyInput = () => {
      const v = clamp(parseFloat(input.value), min, max);
      set(v);
      draw();
    };

    input.addEventListener('input', () => {
      if (suppressSidebarEvents) return;
      applyInput();
    });
    return state;
  }

  const sliders: SliderState[] = [
    makeSlider('Width', -20.0, 50.0, 0.01, 2, () => strokeWidth, (v) => { strokeWidth = v; }),
    makeSlider('Alpha1', 0.0, 1.0, 0.001, 3, () => alpha1, (v) => { alpha1 = v; }),
    makeSlider('Alpha2', 0.0, 1.0, 0.001, 3, () => alpha2, (v) => { alpha2 = v; }),
    makeSlider('Alpha3', 0.0, 1.0, 0.001, 3, () => alpha3, (v) => { alpha3 = v; }),
    makeSlider('Alpha4', 0.0, 1.0, 0.001, 3, () => alpha4, (v) => { alpha4 = v; }),
  ];

  const cbDiv = document.createElement('div');
  cbDiv.className = 'control-group';
  const cb = document.createElement('input');
  cb.type = 'checkbox';
  cb.id = 'rc_invert';
  cb.checked = false;
  cb.addEventListener('change', () => {
    if (suppressSidebarEvents) return;
    invertOrder = cb.checked ? 1 : 0;
    draw();
  });
  const cbLabel = document.createElement('label');
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = ' Invert Z-Order';
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);

  function syncSidebarFromState(): void {
    suppressSidebarEvents = true;
    try {
      for (const s of sliders) {
        const v = s.get();
        s.input.value = String(v);
        s.valueEl.textContent = v.toFixed(s.decimals);
      }
      cb.checked = invertOrder > 0.5;
    } finally {
      suppressSidebarEvents = false;
    }
  }

  function draw() {
    renderToCanvas({ demoName: 'rasterizer_compound', canvas, width: W, height: H,
      params: [strokeWidth, alpha1, alpha2, alpha3, alpha4, invertOrder], timeDisplay: timeEl });
    syncSidebarFromState();
  }

  function canvasPos(e: PointerEvent): { x: number; yAgg: number } {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTopDown = (e.clientY - rect.top) * sy;
    return { x, yAgg: H - yTopDown };
  }

  // AGG control regions in demo coordinates (before CSS flip).
  // Because renderToCanvas uses flipY=true, pointer Y must be inverted.
  const canvasSliderDefs = [
    { x1: 190.0, y1: 5.0, x2: 430.0, y2: 12.0, state: sliders[0] }, // Width
    { x1: 5.0, y1: 5.0, x2: 180.0, y2: 12.0, state: sliders[1] },    // Alpha1
    { x1: 5.0, y1: 25.0, x2: 180.0, y2: 32.0, state: sliders[2] },   // Alpha2
    { x1: 5.0, y1: 45.0, x2: 180.0, y2: 52.0, state: sliders[3] },   // Alpha3
    { x1: 5.0, y1: 65.0, x2: 180.0, y2: 72.0, state: sliders[4] },   // Alpha4
  ];

  function sliderIndexAt(x: number, yAgg: number): number {
    for (let i = 0; i < canvasSliderDefs.length; i += 1) {
      const s = canvasSliderDefs[i];
      const yPad = (s.y2 - s.y1) * 0.8; // matches AGG slider hit area padding closely
      if (x >= s.x1 && x <= s.x2 && yAgg >= s.y1 - yPad && yAgg <= s.y2 + yPad) {
        return i;
      }
    }
    return -1;
  }

  function setSliderFromCanvas(i: number, x: number): void {
    const def = canvasSliderDefs[i];
    const t = clamp((x - def.x1) / (def.x2 - def.x1), 0, 1);
    const v = def.state.min + t * (def.state.max - def.state.min);
    def.state.set(v);
  }

  function checkboxHit(x: number, yAgg: number): boolean {
    // C++ cbox_ctrl at (190, 25), check box is about 13.5x13.5.
    const x1 = 190.0;
    const y1 = 25.0;
    const x2 = 330.0; // include text click area for usability
    const y2 = 40.0;
    return x >= x1 && x <= x2 && yAgg >= y1 && yAgg <= y2;
  }

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    const p = canvasPos(e);
    const si = sliderIndexAt(p.x, p.yAgg);
    if (si >= 0) {
      activeCanvasSlider = si;
      setSliderFromCanvas(si, p.x);
      draw();
      canvas.setPointerCapture(e.pointerId);
      e.preventDefault();
      return;
    }
    if (checkboxHit(p.x, p.yAgg)) {
      invertOrder = invertOrder > 0.5 ? 0 : 1;
      draw();
      e.preventDefault();
    }
  }

  function onPointerMove(e: PointerEvent): void {
    if (activeCanvasSlider < 0 || (e.buttons & 1) === 0) return;
    const p = canvasPos(e);
    setSliderFromCanvas(activeCanvasSlider, p.x);
    draw();
    e.preventDefault();
  }

  function onPointerUp(): void {
    activeCanvasSlider = -1;
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
