import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Molecule Viewer',
    'Molecular structure viewer with rotate/scale/pan — matching C++ mol_view.cpp.',
  );

  const W = 400, H = 400;

  let molIdx = 0;
  let thickness = 1.0;
  let textSize = 1.0;
  let angle = 0.0;
  let scale = 1.0;
  let cx = W / 2;
  let cy = H / 2;

  function draw() {
    renderToCanvas({
      demoName: 'mol_view',
      canvas, width: W, height: H,
      params: [molIdx, thickness, textSize, angle, scale, cx, cy],
      timeDisplay: timeEl,
    });
  }

  // Mouse drag: left=rotate, right=pan
  let dragging = 0; // 0=none, 1=left, 2=right
  let lastX = 0, lastY = 0;
  const onPointerDown = (e: PointerEvent) => {
    canvas.setPointerCapture(e.pointerId);
    const rect = canvas.getBoundingClientRect();
    lastX = e.clientX - rect.left;
    lastY = e.clientY - rect.top;
    if (e.button === 0) dragging = 1;
    else if (e.button === 2) dragging = 2;
    e.preventDefault();
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!dragging) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const dx = mx - lastX;
    const dy = my - lastY;
    if (dragging === 1) {
      angle += dx * 0.5;
      scale = Math.max(0.01, scale + dy * -0.005);
    } else if (dragging === 2) {
      const scaleX = W / rect.width;
      const scaleY = H / rect.height;
      cx += dx * scaleX;
      cy += dy * scaleY;
    }
    lastX = mx;
    lastY = my;
    draw();
  };
  const onPointerUp = () => { dragging = 0; };
  const onContextMenu = (e: Event) => { e.preventDefault(); };
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('contextmenu', onContextMenu);

  // Molecule radio buttons
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Molecule';
  radioDiv.appendChild(radioLabel);
  const molNames = ['Caffeine', 'Aspirin', 'Benzene'];
  molNames.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'mol_select';
    rb.value = String(i);
    rb.checked = i === molIdx;
    rb.addEventListener('change', () => { molIdx = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  const controls: CanvasControl[] = [
    {
      type: 'slider',
      label: 'Bond Thickness',
      min: 0.2, max: 3.0, step: 0.1,
      initial: thickness,
      onChange(v) { thickness = v; draw(); },
    },
    {
      type: 'slider',
      label: 'Text Size',
      min: 0.3, max: 3.0, step: 0.1,
      initial: textSize,
      onChange(v) { textSize = v; draw(); },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag to rotate/scale. Right-drag to pan.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('contextmenu', onContextMenu);
    cleanupCC();
  };
}
