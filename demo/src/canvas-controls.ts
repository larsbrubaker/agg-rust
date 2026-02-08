// Canvas control interaction — hit test AGG-rendered controls and sync with sidebar.
//
// AGG controls are rendered in WASM but have no interactivity there.
// This module provides JS-side mouse handling that detects clicks/drags on
// the canvas controls, updates the corresponding sidebar widget, and triggers re-render.

/** Get mouse position in AGG coordinates (origin bottom-left). */
function aggPos(canvas: HTMLCanvasElement, e: MouseEvent): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: canvas.height - (e.clientY - rect.top) * scaleY,
  };
}

// ============================================================================
// Canvas control descriptors — one per AGG control rendered in WASM
// ============================================================================

export interface CanvasSlider {
  type: 'slider';
  x1: number; y1: number; x2: number; y2: number;
  min: number; max: number;
  /** Sidebar <input type="range"> element to sync. */
  sidebarEl: HTMLInputElement;
  /** Called with the new value when changed via canvas. */
  onChange: (v: number) => void;
}

export interface CanvasCheckbox {
  type: 'checkbox';
  x1: number; y1: number; x2: number; y2: number;
  /** Sidebar <input type="checkbox"> element to sync. */
  sidebarEl: HTMLInputElement;
  onChange: (v: boolean) => void;
}

export interface CanvasRadio {
  type: 'radio';
  x1: number; y1: number; x2: number; y2: number;
  numItems: number;
  /** Sidebar radio <input> elements (one per option). */
  sidebarEls: HTMLInputElement[];
  onChange: (index: number) => void;
}

export type CanvasControl = CanvasSlider | CanvasCheckbox | CanvasRadio;

// ============================================================================
// Mouse handler — attach to a canvas, process all registered controls
// ============================================================================

export function setupCanvasControls(
  canvas: HTMLCanvasElement,
  controls: CanvasControl[],
  redraw: () => void,
): () => void {
  let activeSlider: CanvasSlider | null = null;

  function hitTest(x: number, y: number): CanvasControl | null {
    // AGG slider has border_extra = (y2-y1)/2 expanding the hit area
    for (const c of controls) {
      const extra = c.type === 'slider' ? (c.y2 - c.y1) / 2 : 0;
      if (x >= c.x1 - extra && x <= c.x2 + extra &&
          y >= c.y1 - extra && y <= c.y2 + extra) {
        return c;
      }
    }
    return null;
  }

  function sliderValue(slider: CanvasSlider, x: number): number {
    // Track inner area: xs1 = x1+1, xs2 = x2-1 (border_width = 1)
    const xs1 = slider.x1 + 1;
    const xs2 = slider.x2 - 1;
    let t = (x - xs1) / (xs2 - xs1);
    t = Math.max(0, Math.min(1, t));
    return slider.min + t * (slider.max - slider.min);
  }

  function updateSlider(slider: CanvasSlider, value: number) {
    slider.sidebarEl.value = String(value);
    slider.sidebarEl.dispatchEvent(new Event('input'));
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    const pos = aggPos(canvas, e);
    const ctrl = hitTest(pos.x, pos.y);
    if (!ctrl) return;

    if (ctrl.type === 'slider') {
      activeSlider = ctrl;
      const v = sliderValue(ctrl, pos.x);
      updateSlider(ctrl, v);
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === 'checkbox') {
      const newVal = !ctrl.sidebarEl.checked;
      ctrl.sidebarEl.checked = newVal;
      ctrl.sidebarEl.dispatchEvent(new Event('change'));
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === 'radio') {
      // Determine which item was clicked by y position
      const itemHeight = (ctrl.y2 - ctrl.y1) / ctrl.numItems;
      const idx = Math.floor((pos.y - ctrl.y1) / itemHeight);
      const clamped = Math.max(0, Math.min(ctrl.numItems - 1, idx));
      if (ctrl.sidebarEls[clamped]) {
        ctrl.sidebarEls[clamped].checked = true;
        ctrl.sidebarEls[clamped].dispatchEvent(new Event('change'));
      }
      e.stopPropagation();
      e.preventDefault();
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (!activeSlider) return;
    const pos = aggPos(canvas, e);
    const v = sliderValue(activeSlider, pos.x);
    updateSlider(activeSlider, v);
    e.stopPropagation();
    e.preventDefault();
  }

  function onMouseUp() {
    activeSlider = null;
  }

  // Use capture phase so we can intercept before vertex drag handlers
  canvas.addEventListener('mousedown', onMouseDown, true);
  canvas.addEventListener('mousemove', onMouseMove, true);
  canvas.addEventListener('mouseup', onMouseUp, true);
  canvas.addEventListener('mouseleave', onMouseUp, true);

  return () => {
    canvas.removeEventListener('mousedown', onMouseDown, true);
    canvas.removeEventListener('mousemove', onMouseMove, true);
    canvas.removeEventListener('mouseup', onMouseUp, true);
    canvas.removeEventListener('mouseleave', onMouseUp, true);
  };
}
