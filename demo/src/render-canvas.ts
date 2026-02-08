// Shared rendering helper — takes WASM RGBA output and puts it on a canvas.

import { renderDemo } from './wasm.ts';

export interface RenderOptions {
  demoName: string;
  canvas: HTMLCanvasElement;
  width: number;
  height: number;
  params: number[];
  timeDisplay?: HTMLElement;
}

/**
 * Render a demo to a canvas and display timing.
 */
export function renderToCanvas(opts: RenderOptions): void {
  const { demoName, canvas, width, height, params, timeDisplay } = opts;

  canvas.width = width;
  canvas.height = height;
  canvas.style.transform = 'scaleY(-1)';
  const ctx = canvas.getContext('2d')!;

  const t0 = performance.now();
  const pixels = renderDemo(demoName, width, height, params);
  const t1 = performance.now();

  const imageData = new ImageData(new Uint8ClampedArray(pixels.buffer), width, height);
  ctx.putImageData(imageData, 0, 0);

  if (timeDisplay) {
    timeDisplay.textContent = `${(t1 - t0).toFixed(1)} ms`;
  }
}

/**
 * Create the standard demo page layout with canvas + sidebar controls.
 */
export function createDemoLayout(
  container: HTMLElement,
  title: string,
  description: string,
): { canvas: HTMLCanvasElement; sidebar: HTMLElement; timeEl: HTMLElement } {
  container.innerHTML = `
    <div class="demo-page">
      <div class="demo-header">
        <h2>${title}</h2>
        <p>${description}</p>
      </div>
      <div class="demo-body">
        <div class="demo-canvas-area">
          <canvas id="demo-canvas"></canvas>
          <div class="render-time" id="render-time"></div>
        </div>
        <div class="demo-sidebar" id="demo-sidebar"></div>
      </div>
    </div>
  `;

  return {
    canvas: document.getElementById('demo-canvas') as HTMLCanvasElement,
    sidebar: document.getElementById('demo-sidebar') as HTMLElement,
    timeEl: document.getElementById('render-time') as HTMLElement,
  };
}

/**
 * Create a slider control.
 */
export function addSlider(
  sidebar: HTMLElement,
  label: string,
  min: number,
  max: number,
  value: number,
  step: number,
  onChange: (v: number) => void,
): HTMLInputElement {
  const group = document.createElement('div');
  group.className = 'control-group';
  group.innerHTML = `
    <label class="control-label">${label}</label>
    <input type="range" class="control-slider" min="${min}" max="${max}" value="${value}" step="${step}">
    <span class="control-value">${value}</span>
  `;
  sidebar.appendChild(group);

  const slider = group.querySelector('input')!;
  const display = group.querySelector('.control-value')!;
  slider.addEventListener('input', () => {
    const v = parseFloat(slider.value);
    display.textContent = step >= 1 ? String(Math.round(v)) : v.toFixed(1);
    onChange(v);
  });

  return slider;
}

/**
 * Create a checkbox control.
 */
export function addCheckbox(
  sidebar: HTMLElement,
  label: string,
  initial: boolean,
  onChange: (v: boolean) => void,
): HTMLInputElement {
  const group = document.createElement('div');
  group.className = 'control-group';
  group.innerHTML = `
    <label class="control-checkbox-label">
      <input type="checkbox" ${initial ? 'checked' : ''}>
      <span>${label}</span>
    </label>
  `;
  sidebar.appendChild(group);

  const cb = group.querySelector('input')!;
  cb.addEventListener('change', () => onChange(cb.checked));
  return cb;
}

/**
 * Create a radio button group control.
 */
export function addRadioGroup(
  sidebar: HTMLElement,
  label: string,
  options: string[],
  initialIndex: number,
  onChange: (index: number) => void,
): void {
  const group = document.createElement('div');
  group.className = 'control-group';
  const name = 'radio_' + Math.random().toString(36).slice(2, 8);
  let html = `<label class="control-label">${label}</label><div class="control-radio-group">`;
  for (let i = 0; i < options.length; i++) {
    html += `<label class="control-radio-label">
      <input type="radio" name="${name}" value="${i}" ${i === initialIndex ? 'checked' : ''}>
      <span>${options[i]}</span>
    </label>`;
  }
  html += '</div>';
  group.innerHTML = html;
  sidebar.appendChild(group);

  group.querySelectorAll('input').forEach(radio => {
    radio.addEventListener('change', () => {
      if ((radio as HTMLInputElement).checked) {
        onChange(parseInt((radio as HTMLInputElement).value));
      }
    });
  });
}
