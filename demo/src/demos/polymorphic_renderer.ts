import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Polymorphic Renderer',
    'Same shapes rendered with different pixel formats (RGBA32, RGB24, Gray8) — matching C++ polymorphic_renderer.cpp.',
  );

  const W = 400, H = 330;
  let format = 0;

  function draw() {
    renderToCanvas({
      demoName: 'polymorphic_renderer',
      canvas, width: W, height: H,
      params: [format],
      timeDisplay: timeEl,
    });
  }

  // Radio buttons for format selection
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Pixel Format';
  radioDiv.appendChild(radioLabel);
  const names = ['RGBA32 (4 bpp)', 'RGB24 (3 bpp)', 'Gray8 (1 bpp)'];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'poly_format';
    rb.value = String(i);
    rb.checked = i === format;
    rb.addEventListener('change', () => { format = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Select a pixel format to see the same triangle & circle rendered differently.';
  sidebar.appendChild(hint);

  draw();
}
