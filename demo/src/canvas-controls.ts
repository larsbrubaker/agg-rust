// Canvas control interaction — hit test AGG-rendered controls and sync with sidebar.
//
// AGG controls are rendered in WASM but have no interactivity there.
// This module provides JS-side pointer handling that detects clicks/drags on
// the canvas controls, updates the corresponding sidebar widget, and triggers re-render.
// Uses pointer events with setPointerCapture so slider drags work even when
// the cursor leaves the canvas.

export interface CanvasControlOptions {
  /** Coordinate origin used by control bounds. Defaults to AGG bottom-left. */
  origin?: 'bottom-left' | 'top-left';
}

/** Get pointer position in control coordinate space. */
function aggPos(
  canvas: HTMLCanvasElement,
  e: PointerEvent,
  options: CanvasControlOptions,
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  const yTop = (e.clientY - rect.top) * scaleY;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: options.origin === 'top-left' ? yTop : (canvas.height - yTop),
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

export interface CanvasScale {
  type: 'scale';
  x1: number; y1: number; x2: number; y2: number;
  min: number; max: number;
  minDelta?: number;
  /** Sidebar sliders for value1/value2. */
  sidebarEl1: HTMLInputElement;
  sidebarEl2: HTMLInputElement;
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
  /** AGG rbox text height; affects radio circle hit test. Default 9.0. */
  textHeight?: number;
  /** Sidebar radio <input> elements (one per option). */
  sidebarEls: HTMLInputElement[];
  onChange: (index: number) => void;
}

export interface CanvasButton {
  type: 'button';
  x1: number; y1: number; x2: number; y2: number;
  /** Called when clicked within bounds. */
  onClick: () => void;
}

export type CanvasControl = CanvasSlider | CanvasScale | CanvasCheckbox | CanvasRadio | CanvasButton;

// ============================================================================
// Pointer handler — attach to a canvas, process all registered controls
// ============================================================================

export function setupCanvasControls(
  canvas: HTMLCanvasElement,
  controls: CanvasControl[],
  redraw: () => void,
  options: CanvasControlOptions = {},
): () => void {
  let activeSlider: CanvasSlider | null = null;
  let activeScale: CanvasScale | null = null;
  let activeScaleMode: 'value1' | 'value2' | 'slider' | null = null;
  let scaleDragOffset = 0;

  function hitTest(x: number, y: number): CanvasControl | null {
    // AGG slider has border_extra = (y2-y1)/2 expanding the hit area
    for (const c of controls) {
      const extra = (c.type === 'slider' || c.type === 'scale') ? (c.y2 - c.y1) / 2 : 0;
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

  function scaleInner(scale: CanvasScale): { xs1: number; xs2: number; isHorizontal: boolean; extra: number } {
    const isHorizontal = Math.abs(scale.x2 - scale.x1) > Math.abs(scale.y2 - scale.y1);
    return { xs1: scale.x1 + 1, xs2: scale.x2 - 1, isHorizontal, extra: (scale.y2 - scale.y1) / 2 };
  }

  function scaleRangeValues(scale: CanvasScale): { v1: number; v2: number } {
    const v1 = parseFloat(scale.sidebarEl1.value);
    const v2 = parseFloat(scale.sidebarEl2.value);
    return { v1, v2 };
  }

  function clampScalePair(scale: CanvasScale, v1: number, v2: number): { v1: number; v2: number } {
    const minD = scale.minDelta ?? 0.01;
    const lo = scale.min;
    const hi = scale.max;
    v1 = Math.max(lo, Math.min(v1, hi));
    v2 = Math.max(lo, Math.min(v2, hi));
    if (v2 - v1 < minD) {
      const mid = (v1 + v2) / 2;
      v1 = mid - minD / 2;
      v2 = mid + minD / 2;
      if (v1 < lo) { v1 = lo; v2 = lo + minD; }
      if (v2 > hi) { v2 = hi; v1 = hi - minD; }
    }
    return { v1, v2 };
  }

  function updateScale(scale: CanvasScale, v1: number, v2: number) {
    const pair = clampScalePair(scale, v1, v2);
    scale.sidebarEl1.value = String(pair.v1);
    scale.sidebarEl2.value = String(pair.v2);
    scale.sidebarEl1.dispatchEvent(new Event('input'));
    scale.sidebarEl2.dispatchEvent(new Event('input'));
  }

  function updateSlider(slider: CanvasSlider, value: number) {
    slider.sidebarEl.value = String(value);
    slider.sidebarEl.dispatchEvent(new Event('input'));
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = aggPos(canvas, e, options);
    const ctrl = hitTest(pos.x, pos.y);
    if (!ctrl) return;

    if (ctrl.type === 'slider') {
      activeSlider = ctrl;
      canvas.setPointerCapture(e.pointerId);
      const v = sliderValue(ctrl, pos.x);
      updateSlider(ctrl, v);
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === 'scale') {
      const { xs1, xs2, isHorizontal, extra } = scaleInner(ctrl);
      const { v1, v2 } = scaleRangeValues(ctrl);
      const p1 = isHorizontal ? xs1 + (xs2 - xs1) * ((v1 - ctrl.min) / (ctrl.max - ctrl.min)) : 0;
      const p2 = isHorizontal ? xs1 + (xs2 - xs1) * ((v2 - ctrl.min) / (ctrl.max - ctrl.min)) : 0;
      const py = isHorizontal ? (ctrl.y1 + ctrl.y2) / 2 : 0;
      const pr = isHorizontal ? (ctrl.y2 - ctrl.y1) : (ctrl.x2 - ctrl.x1);
      const ys1 = ctrl.y1 - extra / 2;
      const ys2 = ctrl.y2 + extra / 2;
      const d1 = isHorizontal ? Math.hypot(pos.x - p1, pos.y - py) : Number.POSITIVE_INFINITY;
      const d2 = isHorizontal ? Math.hypot(pos.x - p2, pos.y - py) : Number.POSITIVE_INFINITY;

      if (isHorizontal && pos.x > p1 && pos.x < p2 && pos.y > ys1 && pos.y < ys2) {
        activeScaleMode = 'slider';
        scaleDragOffset = p1 - pos.x;
      } else if (d1 <= pr) {
        activeScaleMode = 'value1';
        scaleDragOffset = p1 - pos.x;
      } else if (d2 <= pr) {
        activeScaleMode = 'value2';
        scaleDragOffset = p2 - pos.x;
      } else {
        activeScaleMode = null;
      }
      if (activeScaleMode) {
        activeScale = ctrl;
        canvas.setPointerCapture(e.pointerId);
        e.stopPropagation();
        e.preventDefault();
      }
    } else if (ctrl.type === 'checkbox') {
      const newVal = !ctrl.sidebarEl.checked;
      ctrl.sidebarEl.checked = newVal;
      ctrl.sidebarEl.dispatchEvent(new Event('change'));
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === 'radio') {
      // Match AGG rbox_ctrl hit testing:
      // click only counts when it's inside an item's radio circle.
      const textHeight = ctrl.textHeight ?? 9.0;
      const dy = textHeight * 2.0;
      const xs1 = ctrl.x1 + 1.0;
      const ys1 = ctrl.y1 + 1.0;
      const cx = xs1 + dy / 1.3;
      const radius = textHeight / 1.5;
      let picked = -1;
      for (let i = 0; i < ctrl.numItems; i++) {
        const cy = ys1 + dy * i + dy / 1.3;
        const dx = pos.x - cx;
        const dyClick = pos.y - cy;
        if (Math.hypot(dx, dyClick) <= radius) {
          picked = i;
          break;
        }
      }
      if (picked >= 0 && ctrl.sidebarEls[picked]) {
        ctrl.sidebarEls[picked].checked = true;
        ctrl.sidebarEls[picked].dispatchEvent(new Event('change'));
      }
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === 'button') {
      ctrl.onClick();
      e.stopPropagation();
      e.preventDefault();
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!activeSlider) return;
    const pos = aggPos(canvas, e, options);
    const v = sliderValue(activeSlider, pos.x);
    updateSlider(activeSlider, v);
    e.stopPropagation();
    e.preventDefault();
  }

  function onPointerMoveScale(e: PointerEvent) {
    if (!activeScale || !activeScaleMode) return;
    const pos = aggPos(canvas, e, options);
    const scale = activeScale;
    const { xs1, xs2, isHorizontal } = scaleInner(scale);
    if (!isHorizontal) return;
    const { v1, v2 } = scaleRangeValues(scale);
    const dv = v2 - v1;
    const toValue = (x: number) => {
      let t = (x - xs1) / (xs2 - xs1);
      t = Math.max(0, Math.min(1, t));
      return scale.min + t * (scale.max - scale.min);
    };
    const x = pos.x + scaleDragOffset;
    if (activeScaleMode === 'value1') {
      updateScale(scale, toValue(x), v2);
    } else if (activeScaleMode === 'value2') {
      updateScale(scale, v1, toValue(x));
    } else {
      const nextV1 = toValue(x);
      updateScale(scale, nextV1, nextV1 + dv);
    }
    e.stopPropagation();
    e.preventDefault();
  }

  function onPointerUp() {
    activeSlider = null;
    activeScale = null;
    activeScaleMode = null;
  }

  // Use capture phase so we can intercept before vertex drag handlers
  canvas.addEventListener('pointerdown', onPointerDown, true);
  canvas.addEventListener('pointermove', onPointerMove, true);
  canvas.addEventListener('pointermove', onPointerMoveScale, true);
  canvas.addEventListener('pointerup', onPointerUp, true);
  canvas.addEventListener('pointercancel', onPointerUp, true);

  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown, true);
    canvas.removeEventListener('pointermove', onPointerMove, true);
    canvas.removeEventListener('pointermove', onPointerMoveScale, true);
    canvas.removeEventListener('pointerup', onPointerUp, true);
    canvas.removeEventListener('pointercancel', onPointerUp, true);
  };
}
