import { createDemoLayout, renderToCanvas, addSlider } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Molecule Viewer',
    'Molecular structure viewer with rotate/scale/pan — matching C++ mol_view.cpp.',
  );

  const W = 400, H = 400;

  let molIdx = 0;
  let thickness = 0.5;
  let textSize = 0.5;
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

  // Mouse drag behavior matches C++ mol_view.cpp:
  // left drag = rotate+scale around center, right drag = pan.
  let dragging = 0; // 0=none, 1=left, 2=right
  let pdx = 0.0;
  let pdy = 0.0;
  let prevScale = 1.0;
  let prevAngle = 0.0;

  function aggPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY,
    };
  }

  const onPointerDown = (e: PointerEvent) => {
    canvas.setPointerCapture(e.pointerId);
    const p = aggPos(e);
    if (e.button === 0) dragging = 1;
    else if (e.button === 2) dragging = 2;
    pdx = cx - p.x;
    pdy = cy - p.y;
    prevScale = scale;
    prevAngle = angle + Math.PI;
    e.preventDefault();
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!dragging) return;
    const p = aggPos(e);
    if (dragging === 1) {
      const dx = p.x - cx;
      const dy = p.y - cy;
      const prevLen = Math.hypot(pdx, pdy);
      if (prevLen > 1e-6) {
        scale = Math.max(0.01, prevScale * (Math.hypot(dx, dy) / prevLen));
      }
      angle = prevAngle + Math.atan2(dy, dx) - Math.atan2(pdy, pdx);
    } else if (dragging === 2) {
      cx = p.x + pdx;
      cy = p.y + pdy;
    }
    draw();
  };
  const onPointerUp = () => { dragging = 0; };
  const onContextMenu = (e: Event) => { e.preventDefault(); };
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);
  canvas.addEventListener('contextmenu', onContextMenu);

  // Molecule radio buttons
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Molecule';
  radioDiv.appendChild(radioLabel);
  const molNames = ['Molecule 1', 'Molecule 2', 'Molecule 3'];
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

  const slThickness = addSlider(sidebar, 'Thickness', 0, 1, thickness, 0.01, v => {
    thickness = v;
    draw();
  });
  const slText = addSlider(sidebar, 'Label Size', 0, 1, textSize, 0.01, v => {
    textSize = v;
    draw();
  });

  const canvasControls: CanvasControl[] = [
    {
      type: 'slider',
      x1: 5, y1: 5, x2: W - 5, y2: 12,
      min: 0, max: 1,
      sidebarEl: slThickness,
      onChange(v) { thickness = v; draw(); },
    },
    {
      type: 'slider',
      x1: 5, y1: 20, x2: W - 5, y2: 27,
      min: 0, max: 1,
      sidebarEl: slText,
      onChange(v) { textSize = v; draw(); },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag to rotate/scale. Right-drag to pan.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
    canvas.removeEventListener('contextmenu', onContextMenu);
    cleanupCC();
  };
}
