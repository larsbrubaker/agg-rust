var __defProp = Object.defineProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, {
      get: all[name],
      enumerable: true,
      configurable: true,
      set: (newValue) => all[name] = () => newValue
    });
};
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);

// src/wasm.ts
async function initWasm() {
  if (wasmModule)
    return;
  const wasmUrl = new URL("./public/pkg/agg_wasm.js", window.location.href).href;
  const mod = await import(wasmUrl);
  await mod.default();
  wasmModule = mod;
}
function getWasm() {
  if (!wasmModule)
    throw new Error("WASM not initialized. Call initWasm() first.");
  return wasmModule;
}
function renderDemo(name, width, height, params) {
  const w = getWasm();
  const result = w.render_demo(name, width, height, new Float64Array(params));
  return new Uint8Array(result);
}
function flashPickVertex(demoName, width, height, params, x, y, radius) {
  const w = getWasm();
  return w.flash_pick_vertex(demoName, width, height, new Float64Array(params), x, y, radius);
}
function flashScreenToShape(demoName, width, height, params, x, y) {
  const w = getWasm();
  const out = w.flash_screen_to_shape(demoName, width, height, new Float64Array(params), x, y);
  return [out[0] ?? x, out[1] ?? y];
}
var wasmModule = null;

// src/render-canvas.ts
function renderToCanvas(opts) {
  const { demoName, canvas, width, height, params, timeDisplay, flipY = true } = opts;
  try {
    const t0 = performance.now();
    const pixels = renderDemo(demoName, width, height, params);
    const t1 = performance.now();
    const expectedBytes = width * height * 4;
    if (pixels.byteLength < expectedBytes) {
      throw new Error(`Pixel buffer too small: got ${pixels.byteLength}, expected ${expectedBytes}`);
    }
    const clamped = pixels.byteLength === expectedBytes ? new Uint8ClampedArray(pixels.buffer, pixels.byteOffset, expectedBytes) : new Uint8ClampedArray(pixels.slice(0, expectedBytes));
    if (canvas.width !== width)
      canvas.width = width;
    if (canvas.height !== height)
      canvas.height = height;
    const expectedTransform = flipY ? "scaleY(-1)" : "none";
    if (canvas.style.transform !== expectedTransform) {
      canvas.style.transform = expectedTransform;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("2D canvas context not available");
    }
    const imageData = new ImageData(clamped, width, height);
    ctx.putImageData(imageData, 0, 0);
    if (timeDisplay) {
      timeDisplay.textContent = `${(t1 - t0).toFixed(1)} ms`;
    }
  } catch (err) {
    console.error(`[renderToCanvas] Failed to render ${demoName}:`, err);
    if (timeDisplay) {
      timeDisplay.textContent = "render failed";
    }
  }
}
function createDemoLayout(container, title, description) {
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
    canvas: document.getElementById("demo-canvas"),
    sidebar: document.getElementById("demo-sidebar"),
    timeEl: document.getElementById("render-time")
  };
}
function addSlider(sidebar, label, min, max, value, step, onChange) {
  const group = document.createElement("div");
  group.className = "control-group";
  group.innerHTML = `
    <label class="control-label">${label}</label>
    <input type="range" class="control-slider" min="${min}" max="${max}" value="${value}" step="${step}">
    <span class="control-value">${value}</span>
  `;
  sidebar.appendChild(group);
  const slider = group.querySelector("input");
  const display = group.querySelector(".control-value");
  slider.addEventListener("input", () => {
    const v = parseFloat(slider.value);
    display.textContent = step >= 1 ? String(Math.round(v)) : v.toFixed(1);
    onChange(v);
  });
  return slider;
}
function addCheckbox(sidebar, label, initial, onChange) {
  const group = document.createElement("div");
  group.className = "control-group";
  group.innerHTML = `
    <label class="control-checkbox-label">
      <input type="checkbox" ${initial ? "checked" : ""}>
      <span>${label}</span>
    </label>
  `;
  sidebar.appendChild(group);
  const cb = group.querySelector("input");
  cb.addEventListener("change", () => onChange(cb.checked));
  return cb;
}
function addRadioGroup(sidebar, label, options, initialIndex, onChange) {
  const group = document.createElement("div");
  group.className = "control-group";
  const name = "radio_" + Math.random().toString(36).slice(2, 8);
  let html = `<label class="control-label">${label}</label><div class="control-radio-group">`;
  for (let i = 0;i < options.length; i++) {
    html += `<label class="control-radio-label">
      <input type="radio" name="${name}" value="${i}" ${i === initialIndex ? "checked" : ""}>
      <span>${options[i]}</span>
    </label>`;
  }
  html += "</div>";
  group.innerHTML = html;
  sidebar.appendChild(group);
  const inputs = [];
  group.querySelectorAll("input").forEach((radio) => {
    inputs.push(radio);
    radio.addEventListener("change", () => {
      if (radio.checked) {
        onChange(parseInt(radio.value));
      }
    });
  });
  return inputs;
}
var init_render_canvas = () => {};

// src/mouse-helpers.ts
function canvasPos(canvas, e) {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: canvas.height - (e.clientY - rect.top) * scaleY
  };
}
function setupVertexDrag(opts) {
  const { canvas, vertices, onDrag } = opts;
  const threshold = opts.threshold ?? 10;
  let dragIdx = -1;
  let dx = 0, dy = 0;
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const pos = canvasPos(canvas, e);
    let bestDist = threshold;
    let bestIdx = -1;
    for (let i = 0;i < vertices.length; i++) {
      const d = Math.hypot(pos.x - vertices[i].x, pos.y - vertices[i].y);
      if (d < bestDist) {
        bestDist = d;
        bestIdx = i;
      }
    }
    if (bestIdx >= 0) {
      dragIdx = bestIdx;
      dx = pos.x - vertices[bestIdx].x;
      dy = pos.y - vertices[bestIdx].y;
      canvas.setPointerCapture(e.pointerId);
    } else if (opts.dragAll && vertices.length >= 3) {
      dragIdx = vertices.length;
      dx = pos.x - vertices[0].x;
      dy = pos.y - vertices[0].y;
      canvas.setPointerCapture(e.pointerId);
    }
  }
  function onPointerMove(e) {
    if (dragIdx < 0)
      return;
    const pos = canvasPos(canvas, e);
    if (dragIdx < vertices.length) {
      vertices[dragIdx].x = pos.x - dx;
      vertices[dragIdx].y = pos.y - dy;
    } else {
      const newX = pos.x - dx;
      const newY = pos.y - dy;
      const ddx = newX - vertices[0].x;
      const ddy = newY - vertices[0].y;
      for (const v of vertices) {
        v.x += ddx;
        v.y += ddy;
      }
    }
    onDrag();
  }
  function onPointerUp() {
    dragIdx = -1;
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
function setupRotateScale(opts) {
  const { canvas, onLeftDrag, onRightDrag } = opts;
  function onPointerDown(e) {
    canvas.setPointerCapture(e.pointerId);
    handlePointer(e);
  }
  function onPointerMove(e) {
    if (e.buttons === 0)
      return;
    handlePointer(e);
  }
  function handlePointer(e) {
    const pos = canvasPos(canvas, e);
    const cx = canvas.width / 2;
    const cy = canvas.height / 2;
    const dx = pos.x - cx;
    const dy = pos.y - cy;
    if (e.buttons & 1) {
      const angle = Math.atan2(dy, dx);
      const scale = Math.hypot(dx, dy) / 100;
      onLeftDrag(angle, scale);
    }
    if (e.buttons & 2 && onRightDrag) {
      onRightDrag(pos.x, pos.y);
    }
  }
  function onContextMenu(e) {
    e.preventDefault();
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("contextmenu", onContextMenu);
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("contextmenu", onContextMenu);
  };
}

// src/canvas-controls.ts
function aggPos(canvas, e, options) {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  const yTop = (e.clientY - rect.top) * scaleY;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: options.origin === "top-left" ? yTop : canvas.height - yTop
  };
}
function setupCanvasControls(canvas, controls, redraw, options = {}) {
  let activeSlider = null;
  let activeScale = null;
  let activeScaleMode = null;
  let scaleDragOffset = 0;
  function hitTest(x, y) {
    for (const c of controls) {
      const extra = c.type === "slider" || c.type === "scale" ? (c.y2 - c.y1) / 2 : 0;
      if (x >= c.x1 - extra && x <= c.x2 + extra && y >= c.y1 - extra && y <= c.y2 + extra) {
        return c;
      }
    }
    return null;
  }
  function sliderValue(slider, x) {
    const xs1 = slider.x1 + 1;
    const xs2 = slider.x2 - 1;
    let t = (x - xs1) / (xs2 - xs1);
    t = Math.max(0, Math.min(1, t));
    return slider.min + t * (slider.max - slider.min);
  }
  function scaleInner(scale) {
    const isHorizontal = Math.abs(scale.x2 - scale.x1) > Math.abs(scale.y2 - scale.y1);
    return { xs1: scale.x1 + 1, xs2: scale.x2 - 1, isHorizontal, extra: (scale.y2 - scale.y1) / 2 };
  }
  function scaleRangeValues(scale) {
    const v1 = parseFloat(scale.sidebarEl1.value);
    const v2 = parseFloat(scale.sidebarEl2.value);
    return { v1, v2 };
  }
  function clampScalePair(scale, v1, v2) {
    const minD = scale.minDelta ?? 0.01;
    const lo = scale.min;
    const hi = scale.max;
    v1 = Math.max(lo, Math.min(v1, hi));
    v2 = Math.max(lo, Math.min(v2, hi));
    if (v2 - v1 < minD) {
      const mid = (v1 + v2) / 2;
      v1 = mid - minD / 2;
      v2 = mid + minD / 2;
      if (v1 < lo) {
        v1 = lo;
        v2 = lo + minD;
      }
      if (v2 > hi) {
        v2 = hi;
        v1 = hi - minD;
      }
    }
    return { v1, v2 };
  }
  function updateScale(scale, v1, v2) {
    const pair = clampScalePair(scale, v1, v2);
    scale.sidebarEl1.value = String(pair.v1);
    scale.sidebarEl2.value = String(pair.v2);
    scale.sidebarEl1.dispatchEvent(new Event("input"));
    scale.sidebarEl2.dispatchEvent(new Event("input"));
  }
  function updateSlider(slider, value) {
    slider.sidebarEl.value = String(value);
    slider.sidebarEl.dispatchEvent(new Event("input"));
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const pos = aggPos(canvas, e, options);
    const ctrl = hitTest(pos.x, pos.y);
    if (!ctrl)
      return;
    if (ctrl.type === "slider") {
      activeSlider = ctrl;
      canvas.setPointerCapture(e.pointerId);
      const v = sliderValue(ctrl, pos.x);
      updateSlider(ctrl, v);
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === "scale") {
      const { xs1, xs2, isHorizontal, extra } = scaleInner(ctrl);
      const { v1, v2 } = scaleRangeValues(ctrl);
      const p1 = isHorizontal ? xs1 + (xs2 - xs1) * ((v1 - ctrl.min) / (ctrl.max - ctrl.min)) : 0;
      const p2 = isHorizontal ? xs1 + (xs2 - xs1) * ((v2 - ctrl.min) / (ctrl.max - ctrl.min)) : 0;
      const py = isHorizontal ? (ctrl.y1 + ctrl.y2) / 2 : 0;
      const pr = isHorizontal ? ctrl.y2 - ctrl.y1 : ctrl.x2 - ctrl.x1;
      const ys1 = ctrl.y1 - extra / 2;
      const ys2 = ctrl.y2 + extra / 2;
      const d1 = isHorizontal ? Math.hypot(pos.x - p1, pos.y - py) : Number.POSITIVE_INFINITY;
      const d2 = isHorizontal ? Math.hypot(pos.x - p2, pos.y - py) : Number.POSITIVE_INFINITY;
      if (isHorizontal && pos.x > p1 && pos.x < p2 && pos.y > ys1 && pos.y < ys2) {
        activeScaleMode = "slider";
        scaleDragOffset = p1 - pos.x;
      } else if (d1 <= pr) {
        activeScaleMode = "value1";
        scaleDragOffset = p1 - pos.x;
      } else if (d2 <= pr) {
        activeScaleMode = "value2";
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
    } else if (ctrl.type === "checkbox") {
      const newVal = !ctrl.sidebarEl.checked;
      ctrl.sidebarEl.checked = newVal;
      ctrl.sidebarEl.dispatchEvent(new Event("change"));
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === "radio") {
      const textHeight = 9;
      const dy = textHeight * 2;
      const xs1 = ctrl.x1 + 1;
      const ys1 = ctrl.y1 + 1;
      const cx = xs1 + dy / 1.3;
      const radius = textHeight / 1.5;
      let picked = -1;
      for (let i = 0;i < ctrl.numItems; i++) {
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
        ctrl.sidebarEls[picked].dispatchEvent(new Event("change"));
      }
      e.stopPropagation();
      e.preventDefault();
    } else if (ctrl.type === "button") {
      ctrl.onClick();
      e.stopPropagation();
      e.preventDefault();
    }
  }
  function onPointerMove(e) {
    if (!activeSlider)
      return;
    const pos = aggPos(canvas, e, options);
    const v = sliderValue(activeSlider, pos.x);
    updateSlider(activeSlider, v);
    e.stopPropagation();
    e.preventDefault();
  }
  function onPointerMoveScale(e) {
    if (!activeScale || !activeScaleMode)
      return;
    const pos = aggPos(canvas, e, options);
    const scale = activeScale;
    const { xs1, xs2, isHorizontal } = scaleInner(scale);
    if (!isHorizontal)
      return;
    const { v1, v2 } = scaleRangeValues(scale);
    const dv = v2 - v1;
    const toValue = (x2) => {
      let t = (x2 - xs1) / (xs2 - xs1);
      t = Math.max(0, Math.min(1, t));
      return scale.min + t * (scale.max - scale.min);
    };
    const x = pos.x + scaleDragOffset;
    if (activeScaleMode === "value1") {
      updateScale(scale, toValue(x), v2);
    } else if (activeScaleMode === "value2") {
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
  canvas.addEventListener("pointerdown", onPointerDown, true);
  canvas.addEventListener("pointermove", onPointerMove, true);
  canvas.addEventListener("pointermove", onPointerMoveScale, true);
  canvas.addEventListener("pointerup", onPointerUp, true);
  canvas.addEventListener("pointercancel", onPointerUp, true);
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown, true);
    canvas.removeEventListener("pointermove", onPointerMove, true);
    canvas.removeEventListener("pointermove", onPointerMoveScale, true);
    canvas.removeEventListener("pointerup", onPointerUp, true);
    canvas.removeEventListener("pointercancel", onPointerUp, true);
  };
}

// src/demos/lion.ts
var exports_lion = {};
__export(exports_lion, {
  init: () => init
});
function init(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Lion", "The classic AGG vector lion — left-drag to rotate & scale, right-drag to skew.");
  let angle = 0;
  let scale = 1;
  let skewX = 0;
  let skewY = 0;
  let alpha = 26;
  const W = 512, H = 400;
  function draw() {
    renderToCanvas({
      demoName: "lion",
      canvas,
      width: W,
      height: H,
      params: [angle, scale, skewX, skewY, alpha],
      timeDisplay: timeEl
    });
  }
  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => {
      angle = a;
      scale = s;
      draw();
    },
    onRightDrag: (x, y) => {
      skewX = x;
      skewY = y;
      draw();
    }
  });
  const slAlpha = addSlider(sidebar, "Alpha", 0, 255, 26, 1, (v) => {
    alpha = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 507, y2: 12, min: 0, max: 255, sidebarEl: slAlpha, onChange: (v) => {
      alpha = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-drag: rotate & scale. Right-drag: skew.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupRS();
    cleanupCC();
  };
}
var init_lion = __esm(() => {
  init_render_canvas();
});

// src/demos/gradients.ts
var exports_gradients = {};
__export(exports_gradients, {
  init: () => init2
});
function init2(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gradients", "6 gradient types with mouse interaction — matching C++ gradients.cpp.");
  const W = 512, H = 400;
  let cx = 350;
  let cy = 280;
  let angle = 0;
  let scale = 1;
  let gradType = 0;
  const scaleX = 1;
  const scaleY = 1;
  let gammaKx1 = 1;
  let gammaKy1 = 1;
  let gammaKx2 = 1;
  let gammaKy2 = 1;
  const splineR = [];
  const splineG = [];
  const splineB = [];
  const splineA = [];
  for (let i = 0;i < 6; i++) {
    const x = i / 5;
    const y = 1 - x;
    splineR.push({ x, y });
    splineG.push({ x, y });
    splineB.push({ x, y });
    splineA.push({ x, y: 1 });
  }
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const flattenSpline = (pts) => pts.flatMap((p) => [p.x, p.y]);
  function draw() {
    renderToCanvas({
      demoName: "gradients",
      canvas,
      width: W,
      height: H,
      params: [
        cx,
        cy,
        angle,
        scale,
        gradType,
        scaleX,
        scaleY,
        gammaKx1,
        gammaKy1,
        gammaKx2,
        gammaKy2,
        ...flattenSpline(splineR),
        ...flattenSpline(splineG),
        ...flattenSpline(splineB),
        ...flattenSpline(splineA)
      ],
      timeDisplay: timeEl
    });
  }
  function canvasPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yRaw = (e.clientY - rect.top) * sy;
    return {
      raw: { x, y: yRaw },
      agg: { x, y: canvas.height - yRaw }
    };
  }
  const splineBoxes = [
    { x1: 210, y1: 10, x2: 460, y2: 45, pts: splineR },
    { x1: 210, y1: 50, x2: 460, y2: 85, pts: splineG },
    { x1: 210, y1: 90, x2: 460, y2: 125, pts: splineB },
    { x1: 210, y1: 130, x2: 460, y2: 165, pts: splineA }
  ];
  let drag = { kind: "none" };
  function inControls(p) {
    if (p.x >= 10 && p.x <= 200 && p.y >= 10 && p.y <= 165)
      return true;
    if (p.x >= 10 && p.x <= 200 && p.y >= 180 && p.y <= 300)
      return true;
    for (const b of splineBoxes) {
      if (p.x >= b.x1 && p.x <= b.x2 && p.y >= b.y1 && p.y <= b.y2)
        return true;
    }
    return false;
  }
  function inSphere(p) {
    const dx = p.x - 350;
    const dy = p.y - 280;
    return dx * dx + dy * dy <= 110 * 110;
  }
  function gammaPoints() {
    const x1 = 10, y1 = 10, x2 = 200, y2 = 165, bw = 2;
    const textH = 8;
    const yc2 = y2 - textH * 2;
    const xs1 = x1 + bw, ys1 = y1 + bw, xs2 = x2 - bw, ys2 = yc2 - bw * 0.5;
    return {
      xs1,
      ys1,
      xs2,
      ys2,
      p1x: xs1 + (xs2 - xs1) * gammaKx1 * 0.25,
      p1y: ys1 + (ys2 - ys1) * gammaKy1 * 0.25,
      p2x: xs2 - (xs2 - xs1) * gammaKx2 * 0.25,
      p2y: ys2 - (ys2 - ys1) * gammaKy2 * 0.25
    };
  }
  function splinePointToCanvas(boxIdx, idx) {
    const b = splineBoxes[boxIdx];
    const bw = 1;
    const xs1 = b.x1 + bw, ys1 = b.y1 + bw, xs2 = b.x2 - bw, ys2 = b.y2 - bw;
    return {
      x: xs1 + (xs2 - xs1) * b.pts[idx].x,
      y: ys1 + (ys2 - ys1) * b.pts[idx].y,
      xs1,
      ys1,
      xs2,
      ys2
    };
  }
  function onPointerDown(e) {
    const pos = canvasPos2(e);
    const useRawForControls = !inControls(pos.agg) && inControls(pos.raw);
    const p = useRawForControls ? pos.raw : pos.agg;
    const btn = e.button;
    canvas.setPointerCapture(e.pointerId);
    const g = gammaPoints();
    const d1 = Math.hypot(p.x - g.p1x, p.y - g.p1y);
    if (d1 <= 8) {
      drag = { kind: "gamma1", pdx: g.p1x - p.x, pdy: g.p1y - p.y, useRaw: useRawForControls };
      return;
    }
    const d2 = Math.hypot(p.x - g.p2x, p.y - g.p2y);
    if (d2 <= 8) {
      drag = { kind: "gamma2", pdx: g.p2x - p.x, pdy: g.p2y - p.y, useRaw: useRawForControls };
      return;
    }
    for (let bi = 0;bi < splineBoxes.length; bi++) {
      for (let i = 0;i < 6; i++) {
        const sp = splinePointToCanvas(bi, i);
        if (Math.hypot(p.x - sp.x, p.y - sp.y) <= 7) {
          drag = { kind: "spline", boxIdx: bi, idx: i, pdx: sp.x - p.x, pdy: sp.y - p.y, useRaw: useRawForControls };
          return;
        }
      }
    }
    if (p.x >= 10 && p.x <= 200 && p.y >= 180 && p.y <= 300) {
      const item = clamp(Math.floor((p.y - 182) / 18), 0, 5);
      gradType = item;
      draw();
      drag = { kind: "none" };
      return;
    }
    if (btn === 0 && inSphere(p)) {
      drag = { kind: "center", lastX: p.x, lastY: p.y };
    } else if (btn === 2 && inSphere(p)) {
      drag = { kind: "rotate", pdx: cx - p.x, pdy: cy - p.y, prevScale: scale, prevAngle: angle + Math.PI };
    }
  }
  function onPointerMove(e) {
    if (drag.kind === "none" || e.buttons === 0)
      return;
    const pos = canvasPos2(e);
    const p = "useRaw" in drag && drag.useRaw ? pos.raw : pos.agg;
    if (drag.kind === "center" && e.buttons & 1) {
      const dx = p.x - drag.lastX;
      const dy = p.y - drag.lastY;
      cx += dx;
      cy += dy;
      drag.lastX = p.x;
      drag.lastY = p.y;
      draw();
      return;
    }
    if (drag.kind === "rotate" && e.buttons & 2) {
      const dx = p.x - cx;
      const dy = p.y - cy;
      const dist = Math.hypot(dx, dy);
      const prevDist = Math.hypot(drag.pdx, drag.pdy);
      if (prevDist > 1)
        scale = drag.prevScale * dist / prevDist;
      angle = drag.prevAngle + Math.atan2(dy, dx) - Math.atan2(drag.pdy, drag.pdx);
      draw();
      return;
    }
    if (drag.kind === "gamma1" || drag.kind === "gamma2") {
      const gp = gammaPoints();
      const x = p.x + drag.pdx;
      const y = p.y + drag.pdy;
      if (drag.kind === "gamma1") {
        gammaKx1 = clamp((x - gp.xs1) * 4 / (gp.xs2 - gp.xs1), 0.001, 1.999);
        gammaKy1 = clamp((y - gp.ys1) * 4 / (gp.ys2 - gp.ys1), 0.001, 1.999);
      } else {
        gammaKx2 = clamp((gp.xs2 - x) * 4 / (gp.xs2 - gp.xs1), 0.001, 1.999);
        gammaKy2 = clamp((gp.ys2 - y) * 4 / (gp.ys2 - gp.ys1), 0.001, 1.999);
      }
      draw();
      return;
    }
    if (drag.kind === "spline") {
      const { boxIdx, idx } = drag;
      const sp = splinePointToCanvas(boxIdx, idx);
      const pts = splineBoxes[boxIdx].pts;
      let nx = clamp((p.x + drag.pdx - sp.xs1) / (sp.xs2 - sp.xs1), 0, 1);
      const ny = clamp((p.y + drag.pdy - sp.ys1) / (sp.ys2 - sp.ys1), 0, 1);
      if (idx === 0)
        nx = 0;
      else if (idx === 5)
        nx = 1;
      else
        nx = clamp(nx, pts[idx - 1].x + 0.001, pts[idx + 1].x - 0.001);
      pts[idx].x = nx;
      pts[idx].y = ny;
      draw();
    }
  }
  function onPointerUp() {
    drag = { kind: "none" };
  }
  function onContextMenu(e) {
    e.preventDefault();
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("contextmenu", onContextMenu);
  addRadioGroup(sidebar, "Gradient Type", ["Radial", "Diamond", "Linear", "XY", "Sqrt XY", "Conic"], 0, (v) => {
    gradType = v;
    draw();
  });
  addSlider(sidebar, "Gamma kx1", 0.001, 1.999, gammaKx1, 0.001, (v) => {
    gammaKx1 = v;
    draw();
  });
  addSlider(sidebar, "Gamma ky1", 0.001, 1.999, gammaKy1, 0.001, (v) => {
    gammaKy1 = v;
    draw();
  });
  addSlider(sidebar, "Gamma kx2", 0.001, 1.999, gammaKx2, 0.001, (v) => {
    gammaKx2 = v;
    draw();
  });
  addSlider(sidebar, "Gamma ky2", 0.001, 1.999, gammaKy2, 0.001, (v) => {
    gammaKy2 = v;
    draw();
  });
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Canvas controls now work: drag gamma/spline points, click gradient type, left-drag center, right-drag rotate/scale.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    canvas.removeEventListener("contextmenu", onContextMenu);
  };
}
var init_gradients = __esm(() => {
  init_render_canvas();
});

// src/demos/gouraud.ts
var exports_gouraud = {};
__export(exports_gouraud, {
  init: () => init3
});
function init3(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gouraud Shading", "6 sub-triangles with draggable vertices — matching C++ gouraud.cpp.");
  const W = 400, H = 320;
  const vertices = [
    { x: 57, y: 60 },
    { x: 369, y: 170 },
    { x: 143, y: 310 }
  ];
  let dilation = 0.175;
  let gamma = 0.809;
  let alpha = 1;
  function draw() {
    renderToCanvas({
      demoName: "gouraud",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        dilation,
        gamma,
        alpha
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    dragAll: true,
    onDrag: draw
  });
  const slDilation = addSlider(sidebar, "Dilation", 0, 1, 0.175, 0.025, (v) => {
    dilation = v;
    draw();
  });
  const slGamma = addSlider(sidebar, "Gamma", 0, 3, 0.809, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const slAlpha = addSlider(sidebar, "Alpha", 0, 1, 1, 0.01, (v) => {
    alpha = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 395, y2: 11, min: 0, max: 1, sidebarEl: slDilation, onChange: (v) => {
      dilation = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 395, y2: 26, min: 0, max: 3, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 35, x2: 395, y2: 41, min: 0, max: 1, sidebarEl: slAlpha, onChange: (v) => {
      alpha = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag vertices or click inside triangle to move all.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_gouraud = __esm(() => {
  init_render_canvas();
});

// src/demos/conv_stroke.ts
var exports_conv_stroke = {};
__export(exports_conv_stroke, {
  init: () => init4
});
function init4(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Conv Stroke", "Stroke joins, caps, and dashed overlay — matching C++ conv_stroke.cpp.");
  const W = 500, H = 330;
  const vertices = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 }
  ];
  let joinType = 2;
  let capType = 2;
  let strokeWidth = 20;
  let miterLimit = 4;
  function draw() {
    renderToCanvas({
      demoName: "conv_stroke",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        joinType,
        capType,
        strokeWidth,
        miterLimit
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    dragAll: true,
    onDrag: draw
  });
  const joinEls = addRadioGroup(sidebar, "Line Join", ["Miter", "Miter Revert", "Round", "Bevel"], 2, (v) => {
    joinType = v;
    draw();
  });
  const capEls = addRadioGroup(sidebar, "Line Cap", ["Butt", "Square", "Round"], 2, (v) => {
    capType = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 3, 40, 20, 0.5, (v) => {
    strokeWidth = v;
    draw();
  });
  const slMiter = addSlider(sidebar, "Miter Limit", 1, 10, 4, 0.1, (v) => {
    miterLimit = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 10, y1: 10, x2: 133, y2: 80, numItems: 4, sidebarEls: joinEls, onChange: (v) => {
      joinType = v;
      draw();
    } },
    { type: "radio", x1: 10, y1: 90, x2: 133, y2: 160, numItems: 3, sidebarEls: capEls, onChange: (v) => {
      capType = v;
      draw();
    } },
    { type: "slider", x1: 140, y1: 14, x2: 490, y2: 22, min: 3, max: 40, sidebarEl: slWidth, onChange: (v) => {
      strokeWidth = v;
      draw();
    } },
    { type: "slider", x1: 140, y1: 34, x2: 490, y2: 42, min: 1, max: 10, sidebarEl: slMiter, onChange: (v) => {
      miterLimit = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 3 vertices or click inside to move all.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_conv_stroke = __esm(() => {
  init_render_canvas();
});

// src/demos/bezier_div.ts
var exports_bezier_div = {};
__export(exports_bezier_div, {
  init: () => init5
});
function init5(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Bezier Div", "Cubic Bezier curve with draggable control points — matching C++ bezier_div.cpp.");
  const W = 655, H = 520;
  const vertices = [
    { x: 170, y: 424 },
    { x: 13, y: 87 },
    { x: 488, y: 423 },
    { x: 26, y: 333 }
  ];
  let strokeWidth = 50;
  let showPoints = true;
  let showOutline = true;
  function draw() {
    renderToCanvas({
      demoName: "bezier_div",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        vertices[3].x,
        vertices[3].y,
        strokeWidth,
        showPoints ? 1 : 0,
        showOutline ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const slWidth = addSlider(sidebar, "Width", -50, 100, 50, 1, (v) => {
    strokeWidth = v;
    draw();
  });
  const cbPts = addCheckbox(sidebar, "Show Points", true, (v) => {
    showPoints = v;
    draw();
  });
  const cbOutline = addCheckbox(sidebar, "Show Outline", true, (v) => {
    showOutline = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 245, y1: 5, x2: 495, y2: 12, min: -50, max: 100, sidebarEl: slWidth, onChange: (v) => {
      strokeWidth = v;
      draw();
    } },
    { type: "checkbox", x1: 250, y1: 15, x2: 400, y2: 30, sidebarEl: cbPts, onChange: (v) => {
      showPoints = v;
      draw();
    } },
    { type: "checkbox", x1: 250, y1: 30, x2: 450, y2: 45, sidebarEl: cbOutline, onChange: (v) => {
      showOutline = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 4 control points. Red = endpoints, green = handles.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_bezier_div = __esm(() => {
  init_render_canvas();
});

// src/demos/circles.ts
var exports_circles = {};
__export(exports_circles, {
  init: () => init6
});
function init6(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Circles", "Random anti-aliased circles — matching C++ circles.cpp.");
  const W = 400, H = 400;
  let zMin = 0.3;
  let zMax = 0.7;
  let size = 0.5;
  let selectivity = 0.5;
  let seed = 1;
  function draw() {
    renderToCanvas({
      demoName: "circles",
      canvas,
      width: W,
      height: H,
      params: [zMin, zMax, size, selectivity, seed],
      timeDisplay: timeEl
    });
  }
  const slZMin = addSlider(sidebar, "Z Min", 0, 1, zMin, 0.01, (v) => {
    zMin = Math.min(v, zMax - 0.01);
    draw();
  });
  const slZMax = addSlider(sidebar, "Z Max", 0, 1, zMax, 0.01, (v) => {
    zMax = Math.max(v, zMin + 0.01);
    draw();
  });
  const slSize = addSlider(sidebar, "Size", 0, 1, size, 0.01, (v) => {
    size = v;
    draw();
  });
  const slSel = addSlider(sidebar, "Selectivity", 0, 1, selectivity, 0.01, (v) => {
    selectivity = v;
    draw();
  });
  addSlider(sidebar, "Seed", 1, 99999, seed, 1, (v) => {
    seed = v;
    draw();
  });
  const canvasControls = [
    { type: "scale", x1: 5, y1: 5, x2: W - 5, y2: 12, min: 0, max: 1, minDelta: 0.01, sidebarEl1: slZMin, sidebarEl2: slZMax },
    { type: "slider", x1: 5, y1: 20, x2: W - 5, y2: 27, min: 0, max: 1, sidebarEl: slSel, onChange: (v) => {
      selectivity = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 35, x2: W - 5, y2: 42, min: 0, max: 1, sidebarEl: slSize, onChange: (v) => {
      size = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return () => cleanupCC();
}
var init_circles = __esm(() => {
  init_render_canvas();
});

// src/demos/rounded_rect.ts
var exports_rounded_rect = {};
__export(exports_rounded_rect, {
  init: () => init7
});
function init7(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Rounded Rect", "Draggable rounded rectangle — matching C++ rounded_rect.cpp.");
  const W = 600, H = 400;
  const vertices = [
    { x: 100, y: 80 },
    { x: 400, y: 280 }
  ];
  let radius = 25;
  let offset = 0;
  let whiteOnBlack = false;
  function draw() {
    renderToCanvas({
      demoName: "rounded_rect",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        radius,
        offset,
        whiteOnBlack ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    onDrag: draw
  });
  const slRadius = addSlider(sidebar, "Radius", 0, 50, 25, 1, (v) => {
    radius = v;
    draw();
  });
  const slOffset = addSlider(sidebar, "Subpixel Offset", -2, 3, 0, 0.1, (v) => {
    offset = v;
    draw();
  });
  const cbWoB = addCheckbox(sidebar, "White on black", false, (v) => {
    whiteOnBlack = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 10, y1: 10, x2: 590, y2: 19, min: 0, max: 50, sidebarEl: slRadius, onChange: (v) => {
      radius = v;
      draw();
    } },
    { type: "slider", x1: 10, y1: 30, x2: 590, y2: 39, min: -2, max: 3, sidebarEl: slOffset, onChange: (v) => {
      offset = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 45, x2: 200, y2: 60, sidebarEl: cbWoB, onChange: (v) => {
      whiteOnBlack = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the two corner points.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_rounded_rect = __esm(() => {
  init_render_canvas();
});

// src/demos/aa_demo.ts
var exports_aa_demo = {};
__export(exports_aa_demo, {
  init: () => init8
});
function init8(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "AA Demo", "Anti-aliasing visualization — enlarged pixel view of a triangle.");
  const W = 600, H = 400;
  const vertices = [
    { x: 57, y: 100 },
    { x: 369, y: 170 },
    { x: 143, y: 310 }
  ];
  let pixelSize = 32;
  function draw() {
    renderToCanvas({
      demoName: "aa_demo",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        pixelSize
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const slPixel = addSlider(sidebar, "Pixel Size", 8, 100, 32, 1, (v) => {
    pixelSize = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 80, y1: 10, x2: W - 10, y2: 19, min: 8, max: 100, sidebarEl: slPixel, onChange: (v) => {
      pixelSize = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag triangle vertices. Each square shows AA coverage.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_aa_demo = __esm(() => {
  init_render_canvas();
});

// src/demos/gamma_correction.ts
var exports_gamma_correction = {};
__export(exports_gamma_correction, {
  init: () => init9
});
function init9(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gamma Correction", "Concentric ellipses with gamma curve visualization — matching C++ gamma_correction.cpp.");
  const W = 400, H = 320;
  let thickness = 1;
  let contrast = 1;
  let gamma = 1;
  function draw() {
    renderToCanvas({
      demoName: "gamma_correction",
      canvas,
      width: W,
      height: H,
      params: [thickness, contrast, gamma],
      timeDisplay: timeEl
    });
  }
  const slThick = addSlider(sidebar, "Thickness", 0, 3, 1, 0.1, (v) => {
    thickness = v;
    draw();
  });
  const slContrast = addSlider(sidebar, "Contrast", 0, 1, 1, 0.01, (v) => {
    contrast = v;
    draw();
  });
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 3, 1, 0.1, (v) => {
    gamma = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 395, y2: 11, min: 0, max: 3, sidebarEl: slThick, onChange: (v) => {
      thickness = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 395, y2: 26, min: 0, max: 1, sidebarEl: slContrast, onChange: (v) => {
      contrast = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 35, x2: 395, y2: 41, min: 0.5, max: 3, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_gamma_correction = __esm(() => {
  init_render_canvas();
});

// src/demos/line_thickness.ts
var exports_line_thickness = {};
__export(exports_line_thickness, {
  init: () => init10
});
function init10(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Line Thickness", "Lines at varying widths — matching C++ line_thickness.cpp.");
  const W = 640, H = 480;
  const vertices = [
    { x: W * 0.05, y: H * 0.5 },
    { x: W * 0.95, y: H * 0.5 }
  ];
  let thickness = 1;
  let blur = 1.5;
  let monochrome = true;
  let invert = false;
  function draw() {
    renderToCanvas({
      demoName: "line_thickness",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        thickness,
        blur,
        monochrome ? 1 : 0,
        invert ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    onDrag: draw
  });
  const slThick = addSlider(sidebar, "Line thickness", 0, 5, 1, 0.1, (v) => {
    thickness = v;
    draw();
  });
  const slBlur = addSlider(sidebar, "Blur radius", 0, 2, 1.5, 0.1, (v) => {
    blur = v;
    draw();
  });
  const cbMono = addCheckbox(sidebar, "Monochrome", true, (v) => {
    monochrome = v;
    draw();
  });
  const cbInvert = addCheckbox(sidebar, "Invert", false, (v) => {
    invert = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 10, y1: 10, x2: 630, y2: 19, min: 0, max: 5, sidebarEl: slThick, onChange: (v) => {
      thickness = v;
      draw();
    } },
    { type: "slider", x1: 10, y1: 30, x2: 630, y2: 39, min: 0, max: 2, sidebarEl: slBlur, onChange: (v) => {
      blur = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 45, x2: 200, y2: 60, sidebarEl: cbMono, onChange: (v) => {
      monochrome = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 65, x2: 200, y2: 80, sidebarEl: cbInvert, onChange: (v) => {
      invert = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag endpoints to tilt lines.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_line_thickness = __esm(() => {
  init_render_canvas();
});

// src/demos/rasterizers.ts
var exports_rasterizers = {};
__export(exports_rasterizers, {
  init: () => init11
});
function init11(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Rasterizers", "Filled and stroked triangle with draggable vertices.");
  const W = 500, H = 330;
  const vertices = [
    { x: 157, y: 60 },
    { x: 369, y: 170 },
    { x: 243, y: 310 }
  ];
  let gammaVal = 0.5;
  let alpha = 1;
  function draw() {
    renderToCanvas({
      demoName: "rasterizers",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        gammaVal,
        alpha
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 15,
    dragAll: true,
    onDrag: draw
  });
  const slGamma = addSlider(sidebar, "Gamma", 0, 1, 0.5, 0.01, (v) => {
    gammaVal = v;
    draw();
  });
  const slAlpha = addSlider(sidebar, "Alpha", 0, 1, 1, 0.01, (v) => {
    alpha = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 1, sidebarEl: slGamma, onChange: (v) => {
      gammaVal = v;
      draw();
    } },
    { type: "slider", x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 1, sidebarEl: slAlpha, onChange: (v) => {
      alpha = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 3 vertices.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_rasterizers = __esm(() => {
  init_render_canvas();
});

// src/demos/conv_contour.ts
var exports_conv_contour = {};
__export(exports_conv_contour, {
  init: () => init12
});
function init12(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Conv Contour", 'Letter "A" with adjustable contour width — matching C++ conv_contour.cpp.');
  const W = 440, H = 330;
  let closeMode = 0;
  let contourWidth = 0;
  let autoDetect = 1;
  function draw() {
    renderToCanvas({
      demoName: "conv_contour",
      canvas,
      width: W,
      height: H,
      params: [closeMode, contourWidth, autoDetect],
      timeDisplay: timeEl
    });
  }
  const radioEls = addRadioGroup(sidebar, "Close", ["Close", "Close CW", "Close CCW"], 0, (v) => {
    closeMode = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", -100, 100, 0, 1, (v) => {
    contourWidth = v;
    draw();
  });
  const cbAuto = addCheckbox(sidebar, "Auto-detect orientation", true, (v) => {
    autoDetect = v ? 1 : 0;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: (v) => {
      closeMode = v;
      draw();
    } },
    { type: "slider", x1: 140, y1: 14, x2: 430, y2: 22, min: -100, max: 100, sidebarEl: slWidth, onChange: (v) => {
      contourWidth = v;
      draw();
    } },
    { type: "checkbox", x1: 140, y1: 25, x2: 430, y2: 40, sidebarEl: cbAuto, onChange: (v) => {
      autoDetect = v ? 1 : 0;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_conv_contour = __esm(() => {
  init_render_canvas();
});

// src/demos/conv_dash.ts
var exports_conv_dash = {};
__export(exports_conv_dash, {
  init: () => init13
});
function init13(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Conv Dash", "Dashed stroke with cap styles — based on C++ conv_dash_marker.cpp.");
  const W = 500, H = 330;
  const vertices = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 }
  ];
  let capType = 0;
  let strokeWidth = 3;
  let closePoly = false;
  let evenOdd = false;
  let smooth = 1;
  function draw() {
    renderToCanvas({
      demoName: "conv_dash",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        capType,
        strokeWidth,
        closePoly ? 1 : 0,
        evenOdd ? 1 : 0,
        smooth
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw
  });
  const radioEls = addRadioGroup(sidebar, "Cap", ["Butt Cap", "Square Cap", "Round Cap"], 0, (v) => {
    capType = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 0.5, 10, 3, 0.5, (v) => {
    strokeWidth = v;
    draw();
  });
  const slSmooth = addSlider(sidebar, "Smooth", 0, 2, 1, 0.1, (v) => {
    smooth = v;
    draw();
  });
  const cbClose = addCheckbox(sidebar, "Close Polygons", false, (v) => {
    closePoly = v;
    draw();
  });
  const cbEO = addCheckbox(sidebar, "Even-Odd Fill", false, (v) => {
    evenOdd = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: (v) => {
      capType = v;
      draw();
    } },
    { type: "slider", x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 10, sidebarEl: slWidth, onChange: (v) => {
      strokeWidth = v;
      draw();
    } },
    { type: "slider", x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 2, sidebarEl: slSmooth, onChange: (v) => {
      smooth = v;
      draw();
    } },
    { type: "checkbox", x1: 140, y1: 25, x2: 290, y2: 40, sidebarEl: cbClose, onChange: (v) => {
      closePoly = v;
      draw();
    } },
    { type: "checkbox", x1: 300, y1: 25, x2: 450, y2: 40, sidebarEl: cbEO, onChange: (v) => {
      evenOdd = v;
      draw();
    } }
  ];
  const cleanupControls = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag triangle vertices. Click canvas controls to interact.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupControls();
  };
}
var init_conv_dash = __esm(() => {
  init_render_canvas();
});

// src/demos/perspective.ts
var exports_perspective = {};
__export(exports_perspective, {
  init: () => init14
});
function init14(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Perspective", "Lion with bilinear/perspective quad transform — matching C++ perspective.cpp.");
  const W = 600, H = 600;
  const ox = (W - 240) / 2;
  const oy = (H - 380) / 2;
  const vertices = [
    { x: ox, y: oy },
    { x: ox + 240, y: oy },
    { x: ox + 240, y: oy + 380 },
    { x: ox, y: oy + 380 }
  ];
  let transType = 0;
  function draw() {
    renderToCanvas({
      demoName: "perspective",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        vertices[3].x,
        vertices[3].y,
        transType
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 20,
    onDrag: draw
  });
  const radioEls = addRadioGroup(sidebar, "Transform", ["Bilinear", "Perspective"], 0, (v) => {
    transType = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 420, y1: 5, x2: 550, y2: 55, numItems: 2, sidebarEls: radioEls, onChange: (v) => {
      transType = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 4 quad corners to warp the lion.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_perspective = __esm(() => {
  init_render_canvas();
});

// src/demos/image_fltr_graph.ts
var exports_image_fltr_graph = {};
__export(exports_image_fltr_graph, {
  init: () => init15
});
function init15(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Filter Graph", "Image filter weight function visualization — matching C++ image_fltr_graph.cpp.");
  const W = 780, H = 300;
  let radius = 4;
  const filterNames = [
    "bilinear",
    "bicubic",
    "spline16",
    "spline36",
    "hanning",
    "hamming",
    "hermite",
    "kaiser",
    "quadric",
    "catrom",
    "gaussian",
    "bessel",
    "mitchell",
    "sinc",
    "lanczos",
    "blackman"
  ];
  const enabled = new Array(16).fill(false);
  function draw() {
    renderToCanvas({
      demoName: "image_fltr_graph",
      canvas,
      width: W,
      height: H,
      params: [radius, ...enabled.map((v) => v ? 1 : 0)],
      timeDisplay: timeEl
    });
  }
  const slRadius = addSlider(sidebar, "Radius", 2, 8, 4, 0.001, (v) => {
    radius = v;
    draw();
  });
  const cbEls = [];
  for (let i = 0;i < 16; i++) {
    const cb = addCheckbox(sidebar, filterNames[i], false, (v) => {
      enabled[i] = v;
      draw();
    });
    cbEls.push(cb);
  }
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 775, y2: 15, min: 2, max: 8, sidebarEl: slRadius, onChange: (v) => {
      radius = v;
      draw();
    } }
  ];
  for (let i = 0;i < 16; i++) {
    const y = 30 + 15 * i;
    canvasControls.push({
      type: "checkbox",
      x1: 8,
      y1: y,
      x2: 120,
      y2: y + 12,
      sidebarEl: cbEls[i],
      onChange: (v) => {
        enabled[i] = v > 0.5;
        draw();
      }
    });
  }
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Enable filters to compare weight curves: red=weight, green=cumulative, blue=normalized.";
  sidebar.appendChild(hint);
  draw();
  return cleanupCC;
}
var init_image_fltr_graph = __esm(() => {
  init_render_canvas();
});

// src/demos/image1.ts
var exports_image1 = {};
__export(exports_image1, {
  init: () => init16
});
function init16(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Affine Transforms", "Original AGG spheres image rotated/scaled through an ellipse with bilinear filtering. Port of image1.cpp.");
  const W = 340, H = 360;
  let angle = 0;
  let scale = 1;
  function draw() {
    renderToCanvas({
      demoName: "image1",
      canvas,
      width: W,
      height: H,
      params: [angle, scale],
      timeDisplay: timeEl
    });
  }
  const slAngle = addSlider(sidebar, "Angle", -180, 180, 0, 1, (v) => {
    angle = v;
    draw();
  });
  const slScale = addSlider(sidebar, "Scale", 0.1, 5, 1, 0.05, (v) => {
    scale = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 300, y2: 12, min: -180, max: 180, sidebarEl: slAngle, onChange: (v) => {
      angle = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 300, y2: 27, min: 0.1, max: 5, sidebarEl: slScale, onChange: (v) => {
      scale = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_image1 = __esm(() => {
  init_render_canvas();
});

// src/demos/image_filters.ts
var exports_image_filters = {};
__export(exports_image_filters, {
  init: () => init17
});
function init17(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Filters", "Iterative image rotation showing filter quality degradation — matching C++ image_filters.cpp.");
  const W = 430, H = 340;
  let filterIdx = 1;
  let stepDeg = 5;
  let normalize = true;
  let radius = 4;
  let numSteps = 0;
  let kpixSec = 0;
  let running = false;
  let animId = 0;
  let runStartTime = 0;
  let runTotalPixels = 0;
  const IMG_PIXELS = 320 * 300;
  function draw(incremental = false) {
    renderToCanvas({
      demoName: "image_filters",
      canvas,
      width: W,
      height: H,
      params: [filterIdx, stepDeg, normalize ? 1 : 0, radius, numSteps, kpixSec, incremental ? 1 : 0],
      timeDisplay: timeEl
    });
  }
  const filterNames = [
    "simple (NN)",
    "bilinear",
    "bicubic",
    "spline16",
    "spline36",
    "hanning",
    "hamming",
    "hermite",
    "kaiser",
    "quadric",
    "catrom",
    "gaussian",
    "bessel",
    "mitchell",
    "sinc",
    "lanczos",
    "blackman"
  ];
  const radioEls = addRadioGroup(sidebar, "Filter", filterNames, 1, (v) => {
    filterIdx = v;
    numSteps = 0;
    kpixSec = 0;
    draw();
  });
  const slStep = addSlider(sidebar, "Step", 1, 10, 5, 0.01, (v) => {
    stepDeg = v;
    draw();
  });
  const slRadius = addSlider(sidebar, "Filter Radius", 2, 8, 4, 0.001, (v) => {
    radius = v;
    draw();
  });
  const cbNorm = addCheckbox(sidebar, "Normalize Filter", true, (v) => {
    normalize = v;
    draw();
  });
  function addButton(parent, label, onClick) {
    const btn = document.createElement("button");
    btn.textContent = label;
    btn.style.cssText = "display:block;margin:4px 0;padding:4px 12px;cursor:pointer;font-size:12px;";
    btn.addEventListener("click", onClick);
    parent.appendChild(btn);
    return btn;
  }
  function doSingleStep() {
    numSteps++;
    draw(true);
  }
  function doRun() {
    if (running)
      return;
    running = true;
    btnRun.textContent = "Running...";
    kpixSec = 0;
    const maxSteps = Math.ceil(360 / stepDeg);
    runStartTime = performance.now();
    runTotalPixels = 0;
    function step() {
      if (numSteps >= maxSteps || !running) {
        running = false;
        btnRun.textContent = "RUN Test!";
        const elapsed = (performance.now() - runStartTime) / 1000;
        if (elapsed > 0 && runTotalPixels > 0) {
          kpixSec = runTotalPixels / 1000 / elapsed;
        }
        draw();
        return;
      }
      numSteps++;
      runTotalPixels += IMG_PIXELS;
      draw(true);
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
  addButton(sidebar, "Single Step", doSingleStep);
  const btnRun = addButton(sidebar, "RUN Test!", doRun);
  addButton(sidebar, "Refresh", doRefresh);
  const canvasControls = [
    {
      type: "slider",
      x1: 115,
      y1: 5,
      x2: 400,
      y2: 11,
      min: 1,
      max: 10,
      sidebarEl: slStep,
      onChange: (v) => {
        stepDeg = v;
        draw();
      }
    },
    {
      type: "slider",
      x1: 115,
      y1: 20,
      x2: 400,
      y2: 26,
      min: 2,
      max: 8,
      sidebarEl: slRadius,
      onChange: (v) => {
        radius = v;
        draw();
      }
    },
    {
      type: "radio",
      x1: 0,
      y1: 0,
      x2: 110,
      y2: 210,
      numItems: 17,
      sidebarEls: radioEls,
      onChange: (idx) => {
        filterIdx = idx;
        numSteps = 0;
        kpixSec = 0;
        draw();
      }
    },
    {
      type: "checkbox",
      x1: 8,
      y1: 215,
      x2: 110,
      y2: 228,
      sidebarEl: cbNorm,
      onChange: (v) => {
        normalize = v;
        draw();
      }
    },
    {
      type: "button",
      x1: 8,
      y1: 230,
      x2: 100,
      y2: 243,
      onClick: doSingleStep
    },
    {
      type: "button",
      x1: 8,
      y1: 245,
      x2: 80,
      y2: 258,
      onClick: doRun
    },
    {
      type: "button",
      x1: 8,
      y1: 265,
      x2: 75,
      y2: 278,
      onClick: doRefresh
    }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Select a filter, then click RUN to see quality degradation over a full 360° rotation. Controls on canvas are interactive.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    running = false;
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
var init_image_filters = __esm(() => {
  init_render_canvas();
});

// src/demos/gradient_focal.ts
var exports_gradient_focal = {};
__export(exports_gradient_focal, {
  init: () => init18
});
function init18(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gradient Focal", "Radial gradient with moveable focal point — matching C++ gradient_focal.cpp.");
  const W = 600, H = 400;
  let focalX = W / 2;
  let focalY = H / 2;
  let gamma = 1;
  function draw() {
    renderToCanvas({
      demoName: "gradient_focal",
      canvas,
      width: W,
      height: H,
      params: [focalX, focalY, gamma],
      timeDisplay: timeEl
    });
  }
  let dragging = false;
  function aggPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: canvas.height - (e.clientY - rect.top) * sy
    };
  }
  function onDown(e) {
    if (e.button !== 0)
      return;
    dragging = true;
    canvas.setPointerCapture(e.pointerId);
    const p = aggPos2(e);
    focalX = p.x;
    focalY = p.y;
    draw();
  }
  function onMove(e) {
    if (!dragging)
      return;
    const p = aggPos2(e);
    focalX = p.x;
    focalY = p.y;
    draw();
  }
  function onUp() {
    dragging = false;
  }
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  canvas.addEventListener("pointercancel", onUp);
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 2.5, 1, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 340, y2: 12, min: 0.5, max: 2.5, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Click/drag to move the focal point.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onDown);
    canvas.removeEventListener("pointermove", onMove);
    canvas.removeEventListener("pointerup", onUp);
    canvas.removeEventListener("pointercancel", onUp);
    cleanupCC();
  };
}
var init_gradient_focal = __esm(() => {
  init_render_canvas();
});

// src/demos/idea.ts
var exports_idea = {};
__export(exports_idea, {
  init: () => init19
});
function init19(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Idea", "Rotating light bulb icon with fill options — matching C++ idea.cpp.");
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
      demoName: "idea",
      canvas,
      width: W,
      height: H,
      params: [angle, evenOdd ? 1 : 0, draft ? 1 : 0, roundoff ? 1 : 0, angleDelta, rotating ? 1 : 0],
      timeDisplay: timeEl
    });
  }
  function animate() {
    angle += angleDelta;
    draw();
    if (rotating)
      animId = requestAnimationFrame(animate);
  }
  function startStop(v) {
    rotating = v;
    if (v) {
      animId = requestAnimationFrame(animate);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }
  const cbRotate = addCheckbox(sidebar, "Rotate", false, (v) => startStop(v));
  const cbEvenOdd = addCheckbox(sidebar, "Even-Odd", false, (v) => {
    evenOdd = v;
    draw();
  });
  const cbDraft = addCheckbox(sidebar, "Draft", false, (v) => {
    draft = v;
    draw();
  });
  const cbRoundoff = addCheckbox(sidebar, "Roundoff", false, (v) => {
    roundoff = v;
    draw();
  });
  const slStep = addSlider(sidebar, "Step (degrees)", 0, 0.1, 0.01, 0.001, (v) => {
    angleDelta = v;
    draw();
  });
  const canvasControls = [
    { type: "checkbox", x1: 10, y1: 3, x2: 55, y2: 16, sidebarEl: cbRotate, onChange: (v) => startStop(v) },
    { type: "checkbox", x1: 60, y1: 3, x2: 125, y2: 16, sidebarEl: cbEvenOdd, onChange: (v) => {
      evenOdd = v;
      draw();
    } },
    { type: "checkbox", x1: 130, y1: 3, x2: 170, y2: 16, sidebarEl: cbDraft, onChange: (v) => {
      draft = v;
      draw();
    } },
    { type: "checkbox", x1: 175, y1: 3, x2: 240, y2: 16, sidebarEl: cbRoundoff, onChange: (v) => {
      roundoff = v;
      draw();
    } },
    { type: "slider", x1: 10, y1: 21, x2: 240, y2: 27, min: 0, max: 0.1, sidebarEl: slStep, onChange: (v) => {
      angleDelta = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return () => {
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
var init_idea = __esm(() => {
  init_render_canvas();
});

// src/demos/graph_test.ts
var exports_graph_test = {};
__export(exports_graph_test, {
  init: () => init20
});
function init20(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Graph Test", "Random graph with 200 nodes and 100 edges — matching C++ graph_test.cpp.");
  const W = 700, H = 530;
  let edgeType = 0;
  let strokeWidth = 2;
  let drawNodes = true;
  let drawEdges = true;
  let draft = false;
  let translucent = false;
  function draw() {
    renderToCanvas({
      demoName: "graph_test",
      canvas,
      width: W,
      height: H,
      params: [edgeType, strokeWidth, drawNodes ? 1 : 0, drawEdges ? 1 : 0, draft ? 1 : 0, translucent ? 1 : 0],
      timeDisplay: timeEl
    });
  }
  const radioEls = addRadioGroup(sidebar, "Edge Type", ["Solid lines", "Bezier curves", "Dashed curves", "Polygons AA", "Polygons Bin"], 0, (v) => {
    edgeType = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 0, 5, 2, 0.1, (v) => {
    strokeWidth = v;
    draw();
  });
  const cbNodes = addCheckbox(sidebar, "Draw Nodes", true, (v) => {
    drawNodes = v;
    draw();
  });
  const cbEdges = addCheckbox(sidebar, "Draw Edges", true, (v) => {
    drawEdges = v;
    draw();
  });
  const cbDraft = addCheckbox(sidebar, "Draft Mode", false, (v) => {
    draft = v;
    draw();
  });
  const cbTranslucent = addCheckbox(sidebar, "Translucent Mode", false, (v) => {
    translucent = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 5, y1: 35, x2: 110, y2: 110, numItems: 5, sidebarEls: radioEls, onChange: (v) => {
      edgeType = v;
      draw();
    } },
    { type: "slider", x1: 190, y1: 8, x2: 390, y2: 15, min: 0, max: 5, sidebarEl: slWidth, onChange: (v) => {
      strokeWidth = v;
      draw();
    } },
    { type: "checkbox", x1: 398, y1: 21, x2: 485, y2: 34, sidebarEl: cbNodes, onChange: (v) => {
      drawNodes = v;
      draw();
    } },
    { type: "checkbox", x1: 488, y1: 21, x2: 575, y2: 34, sidebarEl: cbEdges, onChange: (v) => {
      drawEdges = v;
      draw();
    } },
    { type: "checkbox", x1: 488, y1: 6, x2: 575, y2: 19, sidebarEl: cbDraft, onChange: (v) => {
      draft = v;
      draw();
    } },
    { type: "checkbox", x1: 190, y1: 21, x2: 395, y2: 34, sidebarEl: cbTranslucent, onChange: (v) => {
      translucent = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_graph_test = __esm(() => {
  init_render_canvas();
});

// src/demos/gamma_tuner.ts
var exports_gamma_tuner = {};
__export(exports_gamma_tuner, {
  init: () => init21
});
function init21(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gamma Tuner", "Gradient background + alpha pattern with gamma correction — matching C++ gamma_tuner.cpp.");
  const W = 500, H = 500;
  let gamma = 2.2;
  let r = 1, g = 1, b = 1;
  let pattern = 2;
  function draw() {
    renderToCanvas({
      demoName: "gamma_tuner",
      canvas,
      width: W,
      height: H,
      params: [gamma, r, g, b, pattern],
      timeDisplay: timeEl
    });
  }
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 4, 2.2, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const slR = addSlider(sidebar, "R", 0, 1, 1, 0.01, (v) => {
    r = v;
    draw();
  });
  const slG = addSlider(sidebar, "G", 0, 1, 1, 0.01, (v) => {
    g = v;
    draw();
  });
  const slB = addSlider(sidebar, "B", 0, 1, 1, 0.01, (v) => {
    b = v;
    draw();
  });
  const radioEls = addRadioGroup(sidebar, "Pattern", ["Horizontal", "Vertical", "Checkered"], 2, (v) => {
    pattern = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 345, y2: 11, min: 0.5, max: 4, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 345, y2: 26, min: 0, max: 1, sidebarEl: slR, onChange: (v) => {
      r = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 35, x2: 345, y2: 41, min: 0, max: 1, sidebarEl: slG, onChange: (v) => {
      g = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 50, x2: 345, y2: 56, min: 0, max: 1, sidebarEl: slB, onChange: (v) => {
      b = v;
      draw();
    } },
    { type: "radio", x1: 355, y1: 1, x2: 495, y2: 60, numItems: 3, sidebarEls: radioEls, onChange: (v) => {
      pattern = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_gamma_tuner = __esm(() => {
  init_render_canvas();
});

// src/demos/image_filters2.ts
var exports_image_filters2 = {};
__export(exports_image_filters2, {
  init: () => init22
});
function init22(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Filters 2", "4x4 test image filtered through 17 filter types — matching C++ image_filters2.cpp.");
  const W = 500, H = 340;
  let filterIdx = 1;
  let gamma = 1;
  let radius = 4;
  let normalize = true;
  const filterNames = [
    "simple (NN)",
    "bilinear",
    "bicubic",
    "spline16",
    "spline36",
    "hanning",
    "hamming",
    "hermite",
    "kaiser",
    "quadric",
    "catrom",
    "gaussian",
    "bessel",
    "mitchell",
    "sinc",
    "lanczos",
    "blackman"
  ];
  function draw() {
    renderToCanvas({
      demoName: "image_filters2",
      canvas,
      width: W,
      height: H,
      params: [filterIdx, gamma, radius, normalize ? 1 : 0],
      timeDisplay: timeEl
    });
  }
  const radioEls = addRadioGroup(sidebar, "Filter", filterNames, 1, (v) => {
    filterIdx = v;
    draw();
  });
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 3, 1, 0.001, (v) => {
    gamma = v;
    draw();
  });
  const slRadius = addSlider(sidebar, "Filter Radius", 2, 8, 4, 0.001, (v) => {
    radius = v;
    draw();
  });
  const cbNormalize = addCheckbox(sidebar, "Normalize Filter", true, (v) => {
    normalize = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 0, y1: 0, x2: 110, y2: 210, numItems: 17, sidebarEls: radioEls, onChange: (v) => {
      filterIdx = v;
      draw();
    } },
    { type: "slider", x1: 115, y1: 5, x2: 495, y2: 11, min: 0.5, max: 3, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } },
    { type: "slider", x1: 115, y1: 20, x2: 495, y2: 26, min: 2, max: 8, sidebarEl: slRadius, onChange: (v) => {
      radius = v;
      draw();
    } },
    { type: "checkbox", x1: 8, y1: 215, x2: 180, y2: 228, sidebarEl: cbNormalize, onChange: (v) => {
      normalize = v > 0.5;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Select a filter to see the 4x4 test image scaled to 300x300. Filters 14+ use the radius slider.";
  sidebar.appendChild(hint);
  draw();
  return cleanupCC;
}
var init_image_filters2 = __esm(() => {
  init_render_canvas();
});

// src/demos/conv_dash_marker.ts
var exports_conv_dash_marker = {};
__export(exports_conv_dash_marker, {
  init: () => init23
});
function init23(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Conv Dash Marker", "Dashed stroke with cap styles — matching C++ conv_dash_marker.cpp layout.");
  const W = 500, H = 330;
  const vertices = [
    { x: 157, y: 60 },
    { x: 469, y: 170 },
    { x: 243, y: 310 }
  ];
  let capType = 0;
  let strokeWidth = 3;
  let closePoly = false;
  let evenOdd = false;
  function draw() {
    renderToCanvas({
      demoName: "conv_dash_marker",
      canvas,
      width: W,
      height: H,
      params: [
        vertices[0].x,
        vertices[0].y,
        vertices[1].x,
        vertices[1].y,
        vertices[2].x,
        vertices[2].y,
        capType,
        strokeWidth,
        closePoly ? 1 : 0,
        evenOdd ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const radioEls = addRadioGroup(sidebar, "Cap Style", ["Butt Cap", "Square Cap", "Round Cap"], 0, (v) => {
    capType = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 0, 10, 3, 0.01, (v) => {
    strokeWidth = v;
    draw();
  });
  const cbClose = addCheckbox(sidebar, "Close Polygons", false, (v) => {
    closePoly = v;
    draw();
  });
  const cbEvenOdd = addCheckbox(sidebar, "Even-Odd Fill", false, (v) => {
    evenOdd = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 10, y1: 10, x2: 130, y2: 80, numItems: 3, sidebarEls: radioEls, onChange: (v) => {
      capType = v;
      draw();
    } },
    { type: "slider", x1: 140, y1: 14, x2: 290, y2: 22, min: 0, max: 10, sidebarEl: slWidth, onChange: (v) => {
      strokeWidth = v;
      draw();
    } },
    { type: "checkbox", x1: 140, y1: 34, x2: 290, y2: 47, sidebarEl: cbClose, onChange: (v) => {
      closePoly = v > 0.5;
      draw();
    } },
    { type: "checkbox", x1: 300, y1: 34, x2: 490, y2: 47, sidebarEl: cbEvenOdd, onChange: (v) => {
      evenOdd = v > 0.5;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag triangle vertices. Dashed strokes with arrowhead markers (simplified).";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_conv_dash_marker = __esm(() => {
  init_render_canvas();
});

// src/demos/aa_test.ts
var exports_aa_test = {};
__export(exports_aa_test, {
  init: () => init24
});
function init24(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "AA Test", "Radial dashes, ellipses, gradient lines, and Gouraud triangles — matching C++ aa_test.cpp.");
  const W = 480, H = 350;
  let gamma = 1.6;
  function draw() {
    renderToCanvas({
      demoName: "aa_test",
      canvas,
      width: W,
      height: H,
      params: [gamma],
      timeDisplay: timeEl
    });
  }
  const slGamma = addSlider(sidebar, "Gamma", 0.1, 3, 1.6, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 340, y2: 12, min: 0.1, max: 3, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Anti-aliasing quality test: radial lines, ellipses at varying sizes, gradient lines, Gouraud triangles.";
  sidebar.appendChild(hint);
  draw();
  return cleanupCC;
}
var init_aa_test = __esm(() => {
  init_render_canvas();
});

// src/demos/bspline.ts
var exports_bspline = {};
__export(exports_bspline, {
  init: () => init25
});
function init25(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "B-Spline", "B-spline curve through 6 draggable control points — matching C++ bspline.cpp.");
  const W = 600, H = 600;
  const vertices = [
    { x: 50, y: 50 },
    { x: 150, y: 550 },
    { x: 250, y: 50 },
    { x: 350, y: 550 },
    { x: 450, y: 50 },
    { x: 550, y: 550 }
  ];
  let numPoints = 20;
  let close = false;
  function draw() {
    renderToCanvas({
      demoName: "bspline",
      canvas,
      width: W,
      height: H,
      params: [
        ...vertices.flatMap((v) => [v.x, v.y]),
        numPoints,
        close ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const slPoints = addSlider(sidebar, "Num Points", 1, 40, 20, 1, (v) => {
    numPoints = v;
    draw();
  });
  const cbClose = addCheckbox(sidebar, "Close", false, (v) => {
    close = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 595, y2: 15, min: 1, max: 40, sidebarEl: slPoints, onChange: (v) => {
      numPoints = v;
      draw();
    } },
    { type: "checkbox", x1: 5, y1: 20, x2: 100, y2: 32, sidebarEl: cbClose, onChange: (v) => {
      close = v > 0.5;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 6 control points. Red line = B-spline curve, gray = control polygon.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_bspline = __esm(() => {
  init_render_canvas();
});

// src/demos/image_perspective.ts
var exports_image_perspective = {};
__export(exports_image_perspective, {
  init: () => init26
});
function init26(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Perspective", "Image transformed through affine/bilinear/perspective quad — matching C++ image_perspective.cpp.");
  const W = 600, H = 600;
  const vertices = [
    { x: 100, y: 100 },
    { x: 500, y: 50 },
    { x: 500, y: 500 },
    { x: 100, y: 500 }
  ];
  let transType = 0;
  function draw() {
    renderToCanvas({
      demoName: "image_perspective",
      canvas,
      width: W,
      height: H,
      params: [
        ...vertices.flatMap((v) => [v.x, v.y]),
        transType
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const radioDiv = document.createElement("div");
  radioDiv.className = "control-group";
  const radioLabel = document.createElement("label");
  radioLabel.className = "control-label";
  radioLabel.textContent = "Transform Type";
  radioDiv.appendChild(radioLabel);
  const names = ["Affine Parallelogram", "Bilinear", "Perspective"];
  names.forEach((name, i) => {
    const row = document.createElement("label");
    row.style.display = "block";
    row.style.cursor = "pointer";
    row.style.marginBottom = "2px";
    const rb = document.createElement("input");
    rb.type = "radio";
    rb.name = "img_persp_trans";
    rb.value = String(i);
    rb.checked = i === transType;
    rb.addEventListener("change", () => {
      transType = i;
      draw();
    });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(" " + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);
  const canvasControls = [];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 4 quad corners to transform the image.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_image_perspective = __esm(() => {
  init_render_canvas();
});

// src/demos/alpha_mask.ts
var exports_alpha_mask = {};
__export(exports_alpha_mask, {
  init: () => init27
});
function init27(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Alpha Mask", "Lion with elliptical alpha mask — matching C++ alpha_mask.cpp.");
  const W = 512, H = 400;
  let angle = 0;
  let scale = 1;
  let skewX = 0;
  let skewY = 0;
  let dragging = false;
  let syncingSidebar = false;
  function draw() {
    renderToCanvas({
      demoName: "alpha_mask",
      canvas,
      width: W,
      height: H,
      params: [angle, scale, skewX, skewY],
      timeDisplay: timeEl
    });
  }
  function setFromTransformPoint(x, y) {
    const dx = x - W / 2;
    const dy = y - H / 2;
    angle = Math.atan2(dy, dx);
    scale = Math.max(Math.hypot(dx, dy) / 100, 0.01);
  }
  function pointerToAgg(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTop = (e.clientY - rect.top) * sy;
    return { x, y: H - yTop };
  }
  const angleSlider = addSlider(sidebar, "Angle (rad)", -3.1416, 3.1416, angle, 0.01, (v) => {
    angle = v;
    if (!syncingSidebar)
      draw();
  });
  const scaleSlider = addSlider(sidebar, "Scale", 0.01, 5, scale, 0.01, (v) => {
    scale = v;
    if (!syncingSidebar)
      draw();
  });
  const skewXSlider = addSlider(sidebar, "Skew X", 0, W, skewX, 1, (v) => {
    skewX = v;
    if (!syncingSidebar)
      draw();
  });
  const skewYSlider = addSlider(sidebar, "Skew Y", 0, H, skewY, 1, (v) => {
    skewY = v;
    if (!syncingSidebar)
      draw();
  });
  function syncSidebar() {
    syncingSidebar = true;
    angleSlider.value = String(angle);
    angleSlider.dispatchEvent(new Event("input"));
    scaleSlider.value = String(scale);
    scaleSlider.dispatchEvent(new Event("input"));
    skewXSlider.value = String(skewX);
    skewXSlider.dispatchEvent(new Event("input"));
    skewYSlider.value = String(skewY);
    skewYSlider.dispatchEvent(new Event("input"));
    syncingSidebar = false;
  }
  function applyPointerState(buttonMask, x, y) {
    if (buttonMask & 1) {
      setFromTransformPoint(x, y);
    }
    if (buttonMask & 2) {
      skewX = x;
      skewY = y;
    }
    syncSidebar();
    draw();
  }
  canvas.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 && e.button !== 2)
      return;
    canvas.setPointerCapture(e.pointerId);
    dragging = true;
    const p = pointerToAgg(e);
    const mask = e.buttons || (e.button === 2 ? 2 : 1);
    applyPointerState(mask, p.x, p.y);
    e.preventDefault();
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("pointermove", (e) => {
    if (!dragging)
      return;
    const p = pointerToAgg(e);
    applyPointerState(e.buttons, p.x, p.y);
    e.preventDefault();
  });
  canvas.addEventListener("pointerup", () => {
    dragging = false;
  });
  canvas.addEventListener("pointercancel", () => {
    dragging = false;
  });
  const canvasControls = [];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-drag (bottom-left coords): set angle/scale. Right-drag: set skew from cursor position.";
  sidebar.appendChild(hint);
  syncSidebar();
  draw();
  return cleanupCC;
}
var init_alpha_mask = __esm(() => {
  init_render_canvas();
});

// src/demos/alpha_gradient.ts
var exports_alpha_gradient = {};
__export(exports_alpha_gradient, {
  init: () => init28
});
function init28(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Alpha Gradient", "Gradient with alpha curve control over a random ellipse background — matching C++ alpha_gradient.cpp.");
  const W = 512, H = 400;
  const vertices = [
    { x: 257, y: 60 },
    { x: 369, y: 170 },
    { x: 143, y: 310 }
  ];
  const alphaValues = [0, 0.2, 0.4, 0.6, 0.8, 1];
  function draw() {
    renderToCanvas({
      demoName: "alpha_gradient",
      canvas,
      width: W,
      height: H,
      params: [
        ...vertices.flatMap((v) => [v.x, v.y]),
        ...alphaValues
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const controls = [];
  for (let i = 0;i < 6; i++) {
    controls.push({
      type: "slider",
      label: `Alpha ${i}`,
      min: 0,
      max: 1,
      step: 0.01,
      initial: alphaValues[i],
      onChange(v) {
        alphaValues[i] = v;
        draw();
      }
    });
  }
  const cleanupCC = setupCanvasControls(canvas, controls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 3 triangle vertices. Adjust alpha curve with sliders.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_alpha_gradient = __esm(() => {
  init_render_canvas();
});

// src/demos/image_alpha.ts
var exports_image_alpha = {};
__export(exports_image_alpha, {
  init: () => init29
});
function init29(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Alpha", "Image with brightness-to-alpha mapping over a random ellipse background — matching C++ image_alpha.cpp.");
  const W = 512, H = 400;
  const alphaValues = [1, 1, 1, 0.5, 0.5, 1];
  function draw() {
    renderToCanvas({
      demoName: "image_alpha",
      canvas,
      width: W,
      height: H,
      params: [...alphaValues],
      timeDisplay: timeEl
    });
  }
  const controls = [];
  for (let i = 0;i < 6; i++) {
    controls.push({
      type: "slider",
      label: `Alpha ${i}`,
      min: 0,
      max: 1,
      step: 0.01,
      initial: alphaValues[i],
      onChange(v) {
        alphaValues[i] = v;
        draw();
      }
    });
  }
  const cleanupCC = setupCanvasControls(canvas, controls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Adjust the 6 alpha curve values to control brightness-to-alpha mapping.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupCC();
  };
}
var init_image_alpha = __esm(() => {
  init_render_canvas();
});

// src/demos/alpha_mask3.ts
var exports_alpha_mask3 = {};
__export(exports_alpha_mask3, {
  init: () => init30
});
function init30(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Alpha Mask 3", "Alpha mask polygon clipping with AND/SUB operations — matching C++ alpha_mask3.cpp.");
  const W = 640, H = 520;
  let scenario = 3;
  let operation = 0;
  let mouseX = W / 2;
  let mouseY = H / 2;
  const scenarioLabels = [
    "Two Simple Paths",
    "Closed Stroke",
    "Great Britain and Arrows",
    "Great Britain and Spiral",
    "Spiral and Glyph"
  ];
  const operationLabels = ["AND", "SUB"];
  function draw() {
    renderToCanvas({
      demoName: "alpha_mask3",
      canvas,
      width: W,
      height: H,
      params: [scenario, operation, mouseX, mouseY],
      timeDisplay: timeEl
    });
  }
  const aggMousePos = (e) => {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const yTop = (e.clientY - rect.top) * scaleY;
    return { x, y: H - yTop };
  };
  let dragging = false;
  const onPointerDown = (e) => {
    if (e.button === 0) {
      const p = aggMousePos(e);
      mouseX = p.x;
      mouseY = p.y;
      draw();
      dragging = true;
      canvas.setPointerCapture(e.pointerId);
    }
  };
  const onPointerMove = (e) => {
    if (!dragging || (e.buttons & 1) === 0)
      return;
    const p = aggMousePos(e);
    mouseX = p.x;
    mouseY = p.y;
    draw();
  };
  const onPointerUp = () => {
    dragging = false;
  };
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  const scenarioLabelsUi = [...scenarioLabels].reverse();
  const scenarioInputsUi = addRadioGroup(sidebar, "Polygons", scenarioLabelsUi, scenarioLabels.length - 1 - scenario, (uiIndex) => {
    scenario = scenarioLabels.length - 1 - uiIndex;
    draw();
  });
  const scenarioInputs = new Array(scenarioLabels.length);
  for (let uiIndex = 0;uiIndex < scenarioLabels.length; uiIndex++) {
    const logicalIndex = scenarioLabels.length - 1 - uiIndex;
    scenarioInputs[logicalIndex] = scenarioInputsUi[uiIndex];
  }
  const operationLabelsUi = [...operationLabels].reverse();
  const operationInputsUi = addRadioGroup(sidebar, "Operation", operationLabelsUi, operationLabels.length - 1 - operation, (uiIndex) => {
    operation = operationLabels.length - 1 - uiIndex;
    draw();
  });
  const operationInputs = new Array(operationLabels.length);
  for (let uiIndex = 0;uiIndex < operationLabels.length; uiIndex++) {
    const logicalIndex = operationLabels.length - 1 - uiIndex;
    operationInputs[logicalIndex] = operationInputsUi[uiIndex];
  }
  const controls = [
    {
      type: "radio",
      x1: 5,
      y1: 5,
      x2: 210,
      y2: 110,
      numItems: 5,
      sidebarEls: scenarioInputs,
      onChange: (index) => {
        scenario = index;
        draw();
      }
    },
    {
      type: "radio",
      x1: 555,
      y1: 5,
      x2: 635,
      y2: 55,
      numItems: 2,
      sidebarEls: operationInputs,
      onChange: (index) => {
        operation = index;
        draw();
      }
    }
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-click or drag on canvas to move shapes. Canvas and sidebar controls are synchronized.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    cleanupCC();
  };
}
var init_alpha_mask3 = __esm(() => {
  init_render_canvas();
});

// src/demos/image_transforms.ts
var exports_image_transforms = {};
__export(exports_image_transforms, {
  init: () => init31
});
function init31(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Transforms", "Star polygon textured with image through 7 transform modes — matching C++ image_transforms.cpp.");
  const W = 430, H = 340;
  let polyAngle = 0;
  let polyScale = 1;
  let imgAngle = 0;
  let imgScale = 1;
  let exampleIdx = 1;
  function draw() {
    renderToCanvas({
      demoName: "image_transforms",
      canvas,
      width: W,
      height: H,
      params: [polyAngle, polyScale, imgAngle, imgScale, exampleIdx, W / 2, H / 2, W / 2, H / 2],
      timeDisplay: timeEl
    });
  }
  const radioDiv = document.createElement("div");
  radioDiv.className = "control-group";
  const radioLabel = document.createElement("label");
  radioLabel.className = "control-label";
  radioLabel.textContent = "Transform Example";
  radioDiv.appendChild(radioLabel);
  const names = [
    "1: Rotate around (img_cx, img_cy)",
    "2: Plus translate to center",
    "3: Image in polygon coords",
    "4: Image in polygon + rotate",
    "5: Image in polygon + rotate + scale",
    "6: Rotate image + polygon same center",
    "7: Rotate image + polygon separately"
  ];
  names.forEach((name, i) => {
    const row = document.createElement("label");
    row.style.display = "block";
    row.style.cursor = "pointer";
    row.style.marginBottom = "2px";
    row.style.fontSize = "12px";
    const rb = document.createElement("input");
    rb.type = "radio";
    rb.name = "img_trans_example";
    rb.value = String(i + 1);
    rb.checked = i + 1 === exampleIdx;
    rb.addEventListener("change", () => {
      exampleIdx = i + 1;
      draw();
    });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(" " + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);
  const controls = [
    {
      type: "slider",
      label: "Polygon Angle",
      min: -180,
      max: 180,
      step: 1,
      initial: polyAngle,
      onChange(v) {
        polyAngle = v;
        draw();
      }
    },
    {
      type: "slider",
      label: "Polygon Scale",
      min: 0.1,
      max: 5,
      step: 0.05,
      initial: polyScale,
      onChange(v) {
        polyScale = v;
        draw();
      }
    },
    {
      type: "slider",
      label: "Image Angle",
      min: -180,
      max: 180,
      step: 1,
      initial: imgAngle,
      onChange(v) {
        imgAngle = v;
        draw();
      }
    },
    {
      type: "slider",
      label: "Image Scale",
      min: 0.1,
      max: 5,
      step: 0.05,
      initial: imgScale,
      onChange(v) {
        imgScale = v;
        draw();
      }
    }
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Select transform example and adjust angles/scales.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupCC();
  };
}
var init_image_transforms = __esm(() => {
  init_render_canvas();
});

// src/demos/mol_view.ts
var exports_mol_view = {};
__export(exports_mol_view, {
  init: () => init32
});
function init32(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Molecule Viewer", "Molecular structure viewer with rotate/scale/pan — matching C++ mol_view.cpp.");
  const W = 400, H = 400;
  let molIdx = 0;
  let thickness = 0.5;
  let textSize = 0.5;
  let angle = 0;
  let scale = 1;
  let cx = W / 2;
  let cy = H / 2;
  function draw() {
    renderToCanvas({
      demoName: "mol_view",
      canvas,
      width: W,
      height: H,
      params: [molIdx, thickness, textSize, angle, scale, cx, cy],
      timeDisplay: timeEl
    });
  }
  let dragging = 0;
  let pdx = 0;
  let pdy = 0;
  let prevScale = 1;
  let prevAngle = 0;
  function aggPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY
    };
  }
  const onPointerDown = (e) => {
    canvas.setPointerCapture(e.pointerId);
    const p = aggPos2(e);
    if (e.button === 0)
      dragging = 1;
    else if (e.button === 2)
      dragging = 2;
    pdx = cx - p.x;
    pdy = cy - p.y;
    prevScale = scale;
    prevAngle = angle + Math.PI;
    e.preventDefault();
  };
  const onPointerMove = (e) => {
    if (!dragging)
      return;
    const p = aggPos2(e);
    if (dragging === 1) {
      const dx = p.x - cx;
      const dy = p.y - cy;
      const prevLen = Math.hypot(pdx, pdy);
      if (prevLen > 0.000001) {
        scale = Math.max(0.01, prevScale * (Math.hypot(dx, dy) / prevLen));
      }
      angle = prevAngle + Math.atan2(dy, dx) - Math.atan2(pdy, pdx);
    } else if (dragging === 2) {
      cx = p.x + pdx;
      cy = p.y + pdy;
    }
    draw();
  };
  const onPointerUp = () => {
    dragging = 0;
  };
  const onContextMenu = (e) => {
    e.preventDefault();
  };
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("contextmenu", onContextMenu);
  const radioDiv = document.createElement("div");
  radioDiv.className = "control-group";
  const radioLabel = document.createElement("label");
  radioLabel.className = "control-label";
  radioLabel.textContent = "Molecule";
  radioDiv.appendChild(radioLabel);
  const molNames = ["Molecule 1", "Molecule 2", "Molecule 3"];
  molNames.forEach((name, i) => {
    const row = document.createElement("label");
    row.style.display = "block";
    row.style.cursor = "pointer";
    row.style.marginBottom = "2px";
    const rb = document.createElement("input");
    rb.type = "radio";
    rb.name = "mol_select";
    rb.value = String(i);
    rb.checked = i === molIdx;
    rb.addEventListener("change", () => {
      molIdx = i;
      draw();
    });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(" " + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);
  const slThickness = addSlider(sidebar, "Thickness", 0, 1, thickness, 0.01, (v) => {
    thickness = v;
    draw();
  });
  const slText = addSlider(sidebar, "Label Size", 0, 1, textSize, 0.01, (v) => {
    textSize = v;
    draw();
  });
  const canvasControls = [
    {
      type: "slider",
      x1: 5,
      y1: 5,
      x2: W - 5,
      y2: 12,
      min: 0,
      max: 1,
      sidebarEl: slThickness,
      onChange(v) {
        thickness = v;
        draw();
      }
    },
    {
      type: "slider",
      x1: 5,
      y1: 20,
      x2: W - 5,
      y2: 27,
      min: 0,
      max: 1,
      sidebarEl: slText,
      onChange(v) {
        textSize = v;
        draw();
      }
    }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-drag to rotate/scale. Right-drag to pan.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    canvas.removeEventListener("contextmenu", onContextMenu);
    cleanupCC();
  };
}
var init_mol_view = __esm(() => {
  init_render_canvas();
});

// src/demos/raster_text.ts
var exports_raster_text = {};
__export(exports_raster_text, {
  init: () => init33
});
function init33(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Raster Text", "All 34 embedded bitmap fonts rendered with sample text — matching C++ raster_text.cpp.");
  const W = 640, H = 480;
  function draw() {
    renderToCanvas({
      demoName: "raster_text",
      canvas,
      width: W,
      height: H,
      params: [],
      timeDisplay: timeEl
    });
  }
  draw();
}
var init_raster_text = __esm(() => {
  init_render_canvas();
});

// src/demos/gamma_ctrl.ts
var exports_gamma_ctrl = {};
__export(exports_gamma_ctrl, {
  init: () => init34
});
function init34(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gamma Control", "Interactive gamma spline widget with stroked ellipses — matching C++ gamma_ctrl.cpp.");
  const W = 500, H = 400;
  let kx1 = 1, ky1 = 1, kx2 = 1, ky2 = 1;
  function draw() {
    renderToCanvas({
      demoName: "gamma_ctrl",
      canvas,
      width: W,
      height: H,
      params: [kx1, ky1, kx2, ky2],
      timeDisplay: timeEl
    });
  }
  addSlider(sidebar, "kx1", 0.001, 1.999, 1, 0.01, (v) => {
    kx1 = v;
    draw();
  });
  addSlider(sidebar, "ky1", 0.001, 1.999, 1, 0.01, (v) => {
    ky1 = v;
    draw();
  });
  addSlider(sidebar, "kx2", 0.001, 1.999, 1, 0.01, (v) => {
    kx2 = v;
    draw();
  });
  addSlider(sidebar, "ky2", 0.001, 1.999, 1, 0.01, (v) => {
    ky2 = v;
    draw();
  });
  draw();
}
var init_gamma_ctrl = __esm(() => {
  init_render_canvas();
});

// src/demos/trans_polar.ts
var exports_trans_polar = {};
__export(exports_trans_polar, {
  init: () => init35
});
function init35(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Polar Transform", "Slider control warped through polar coordinates — matching C++ trans_polar.cpp.");
  const W = 600, H = 400;
  let value = 32;
  let spiral = 0;
  let baseY = 120;
  function draw() {
    renderToCanvas({
      demoName: "trans_polar",
      canvas,
      width: W,
      height: H,
      params: [value, spiral, baseY],
      timeDisplay: timeEl
    });
  }
  const slVal = addSlider(sidebar, "Value", 0, 100, 32, 1, (v) => {
    value = v;
    draw();
  });
  const slSpiral = addSlider(sidebar, "Spiral", -0.1, 0.1, 0, 0.001, (v) => {
    spiral = v;
    draw();
  });
  const slBaseY = addSlider(sidebar, "Base Y", 50, 200, 120, 1, (v) => {
    baseY = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 10, y1: 10, x2: 590, y2: 17, min: 0, max: 100, sidebarEl: slVal, onChange: (v) => {
      value = v;
      draw();
    } },
    { type: "slider", x1: 10, y1: 30, x2: 590, y2: 37, min: -0.1, max: 0.1, sidebarEl: slSpiral, onChange: (v) => {
      spiral = v;
      draw();
    } },
    { type: "slider", x1: 10, y1: 50, x2: 590, y2: 57, min: 50, max: 200, sidebarEl: slBaseY, onChange: (v) => {
      baseY = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_trans_polar = __esm(() => {
  init_render_canvas();
});

// src/demos/multi_clip.ts
var exports_multi_clip = {};
__export(exports_multi_clip, {
  init: () => init36
});
function init36(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Multi Clip", "Lion rendered through N×N clip regions with random shapes — matching C++ multi_clip.cpp.");
  const W = 512, H = 400;
  let n = 4;
  let angle = 0;
  let scale = 1;
  function draw() {
    renderToCanvas({
      demoName: "multi_clip",
      canvas,
      width: W,
      height: H,
      params: [n, angle, scale],
      timeDisplay: timeEl
    });
  }
  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => {
      angle = a;
      scale = s;
      draw();
    }
  });
  const slN = addSlider(sidebar, "N", 2, 10, 4, 1, (v) => {
    n = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 150, y2: 12, min: 2, max: 10, sidebarEl: slN, onChange: (v) => {
      n = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-drag: rotate & scale.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupRS();
    cleanupCC();
  };
}
var init_multi_clip = __esm(() => {
  init_render_canvas();
});

// src/demos/simple_blur.ts
var exports_simple_blur = {};
__export(exports_simple_blur, {
  init: () => init37
});
function init37(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Simple Blur", "Lion with 3×3 box blur — left half original, right half blurred. Matching C++ simple_blur.cpp.");
  const W = 512, H = 400;
  let angle = 0;
  let scale = 1;
  function draw() {
    renderToCanvas({
      demoName: "simple_blur",
      canvas,
      width: W,
      height: H,
      params: [angle, scale],
      timeDisplay: timeEl
    });
  }
  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => {
      angle = a;
      scale = s;
      draw();
    }
  });
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Left-drag: rotate & scale.";
  sidebar.appendChild(hint);
  draw();
  return cleanupRS;
}
var init_simple_blur = __esm(() => {
  init_render_canvas();
});

// src/demos/blur.ts
var exports_blur = {};
__export(exports_blur, {
  init: () => init38
});
function init38(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Blur", "Stack blur and recursive blur on colored shapes — matching C++ blur.cpp.");
  const W = 440, H = 330;
  let radius = 15;
  let method = 0;
  let channelR = false;
  let channelG = true;
  let channelB = false;
  const shadowQuad = [
    { x: 174.24, y: 86.28 },
    { x: 336.76, y: 86.28 },
    { x: 336.76, y: 274.16 },
    { x: 174.24, y: 274.16 }
  ];
  function draw() {
    renderToCanvas({
      demoName: "blur",
      canvas,
      width: W,
      height: H,
      params: [
        radius,
        method,
        channelR ? 1 : 0,
        channelG ? 1 : 0,
        channelB ? 1 : 0,
        shadowQuad[0].x,
        shadowQuad[0].y,
        shadowQuad[1].x,
        shadowQuad[1].y,
        shadowQuad[2].x,
        shadowQuad[2].y,
        shadowQuad[3].x,
        shadowQuad[3].y
      ],
      timeDisplay: timeEl
    });
  }
  const slRadius = addSlider(sidebar, "Blur Radius", 0, 40, 15, 0.01, (v) => {
    radius = v;
    draw();
  });
  const methodRadios = addRadioGroup(sidebar, "Method", ["Stack blur", "Recursive blur", "Channels"], method, (i) => {
    method = i;
    draw();
  });
  const cbRed = addCheckbox(sidebar, "Red", channelR, (v) => {
    channelR = v;
    draw();
  });
  const cbGreen = addCheckbox(sidebar, "Green", channelG, (v) => {
    channelG = v;
    draw();
  });
  const cbBlue = addCheckbox(sidebar, "Blue", channelB, (v) => {
    channelB = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 140, y1: 14, x2: 430, y2: 22, min: 0, max: 40, sidebarEl: slRadius, onChange: (v) => {
      radius = v;
      draw();
    } },
    { type: "radio", x1: 10, y1: 10, x2: 130, y2: 70, numItems: 3, sidebarEls: methodRadios, onChange: (i) => {
      method = i;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 80, x2: 95, y2: 92, sidebarEl: cbRed, onChange: (v) => {
      channelR = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 95, x2: 95, y2: 107, sidebarEl: cbGreen, onChange: (v) => {
      channelG = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 110, x2: 95, y2: 122, sidebarEl: cbBlue, onChange: (v) => {
      channelB = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  function aggPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY
    };
  }
  function inControlBounds(p) {
    for (const c of canvasControls) {
      const extra = c.type === "slider" || c.type === "scale" ? (c.y2 - c.y1) / 2 : 0;
      if (p.x >= c.x1 - extra && p.x <= c.x2 + extra && p.y >= c.y1 - extra && p.y <= c.y2 + extra) {
        return true;
      }
    }
    return false;
  }
  function pointInPoly(p, poly) {
    let inside = false;
    let j = poly.length - 1;
    for (let i = 0;i < poly.length; i++) {
      const pi = poly[i];
      const pj = poly[j];
      const intersects = pi.y > p.y !== pj.y > p.y && p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y || 0.000000000001) + pi.x;
      if (intersects)
        inside = !inside;
      j = i;
    }
    return inside;
  }
  let drag = null;
  function nearestVertex(p) {
    let best = null;
    const threshold = 10;
    for (let i = 0;i < shadowQuad.length; i++) {
      const d = Math.hypot(p.x - shadowQuad[i].x, p.y - shadowQuad[i].y);
      if (d <= threshold && (!best || d < best.d)) {
        best = { idx: i, d };
      }
    }
    return best;
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const p = aggPos2(e);
    if (inControlBounds(p))
      return;
    const near = nearestVertex(p);
    if (near) {
      drag = {
        kind: "vertex",
        idx: near.idx,
        dx: p.x - shadowQuad[near.idx].x,
        dy: p.y - shadowQuad[near.idx].y
      };
      canvas.setPointerCapture(e.pointerId);
      return;
    }
    if (pointInPoly(p, shadowQuad)) {
      drag = { kind: "all", lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
    }
  }
  function onPointerMove(e) {
    if (!drag)
      return;
    const p = aggPos2(e);
    if (drag.kind === "vertex") {
      shadowQuad[drag.idx].x = p.x - drag.dx;
      shadowQuad[drag.idx].y = p.y - drag.dy;
    } else {
      const dx = p.x - drag.lastX;
      const dy = p.y - drag.lastY;
      for (const v of shadowQuad) {
        v.x += dx;
        v.y += dy;
      }
      drag.lastX = p.x;
      drag.lastY = p.y;
    }
    draw();
  }
  function onPointerUp() {
    drag = null;
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
var init_blur = __esm(() => {
  init_render_canvas();
});

// src/demos/trans_curve1.ts
var exports_trans_curve1 = {};
__export(exports_trans_curve1, {
  init: () => init39
});
function init39(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Text Along Curve 1", "TrueType text warped along a B-spline curve using trans_single_path — matching C++ trans_curve1_ft.cpp.");
  const W = 600, H = 600;
  const initPts = [
    { x: 50, y: 50 },
    { x: 170, y: 130 },
    { x: 230, y: 270 },
    { x: 370, y: 330 },
    { x: 430, y: 470 },
    { x: 550, y: 550 }
  ];
  const vertices = initPts.map((p) => ({ ...p }));
  let numPoints = 200;
  let preserveXScale = true;
  let fixedLength = true;
  let closePath = false;
  let animating = false;
  let animId = 0;
  const dx = [0, 0, 0, 0, 0, 0];
  const dy = [0, 0, 0, 0, 0, 0];
  function draw() {
    renderToCanvas({
      demoName: "trans_curve1",
      canvas,
      width: W,
      height: H,
      params: [
        numPoints,
        ...vertices.flatMap((v) => [v.x, v.y]),
        preserveXScale ? 1 : 0,
        fixedLength ? 1 : 0,
        closePath ? 1 : 0,
        animating ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  function movePoint(i) {
    if (vertices[i].x < 0) {
      vertices[i].x = 0;
      dx[i] = -dx[i];
    }
    if (vertices[i].x > W) {
      vertices[i].x = W;
      dx[i] = -dx[i];
    }
    if (vertices[i].y < 0) {
      vertices[i].y = 0;
      dy[i] = -dy[i];
    }
    if (vertices[i].y > H) {
      vertices[i].y = H;
      dy[i] = -dy[i];
    }
    vertices[i].x += dx[i];
    vertices[i].y += dy[i];
  }
  function animateFrame() {
    for (let i = 0;i < 6; i++) {
      movePoint(i);
    }
    draw();
    if (animating)
      animId = requestAnimationFrame(animateFrame);
  }
  function startAnimation(v) {
    animating = v;
    if (v) {
      for (let i = 0;i < 6; i++) {
        vertices[i].x = initPts[i].x;
        vertices[i].y = initPts[i].y;
        dx[i] = (Math.random() * 1000 - 500) * 0.01;
        dy[i] = (Math.random() * 1000 - 500) * 0.01;
      }
      animId = requestAnimationFrame(animateFrame);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const slPoints = addSlider(sidebar, "Num Points", 10, 400, 200, 10, (v) => {
    numPoints = v;
    draw();
  });
  const cbClose = addCheckbox(sidebar, "Close", closePath, (v) => {
    closePath = v;
    draw();
  });
  const cbPreserve = addCheckbox(sidebar, "Preserve X scale", preserveXScale, (v) => {
    preserveXScale = v;
    draw();
  });
  const cbFixed = addCheckbox(sidebar, "Fixed Length", fixedLength, (v) => {
    fixedLength = v;
    draw();
  });
  const cbAnimate = addCheckbox(sidebar, "Animate", false, (v) => startAnimation(v));
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 340, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: (v) => {
      numPoints = v;
      draw();
    } },
    { type: "checkbox", x1: 350, y1: 5, x2: 455, y2: 19, sidebarEl: cbClose, onChange: (v) => {
      closePath = v;
      draw();
    } },
    { type: "checkbox", x1: 460, y1: 5, x2: 595, y2: 19, sidebarEl: cbPreserve, onChange: (v) => {
      preserveXScale = v;
      draw();
    } },
    { type: "checkbox", x1: 350, y1: 25, x2: 455, y2: 39, sidebarEl: cbFixed, onChange: (v) => {
      fixedLength = v;
      draw();
    } },
    { type: "checkbox", x1: 460, y1: 25, x2: 560, y2: 39, sidebarEl: cbAnimate, onChange: (v) => startAnimation(v) }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 6 control points to reshape the curve. Toggle Animate for bouncing points.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    animating = false;
    cancelAnimationFrame(animId);
    cleanupDrag();
    cleanupCC();
  };
}
var init_trans_curve1 = __esm(() => {
  init_render_canvas();
});

// src/demos/trans_curve2.ts
var exports_trans_curve2 = {};
__export(exports_trans_curve2, {
  init: () => init40
});
function init40(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Text Along Curve 2", "TrueType text warped between two B-spline curves using trans_double_path — matching C++ trans_curve2_ft.cpp.");
  const W = 600, H = 600;
  const initPoly1 = [
    { x: 60, y: 40 },
    { x: 180, y: 120 },
    { x: 240, y: 260 },
    { x: 380, y: 320 },
    { x: 440, y: 460 },
    { x: 560, y: 540 }
  ];
  const initPoly2 = [
    { x: 40, y: 60 },
    { x: 160, y: 140 },
    { x: 220, y: 280 },
    { x: 360, y: 340 },
    { x: 420, y: 480 },
    { x: 540, y: 560 }
  ];
  const poly1 = initPoly1.map((p) => ({ ...p }));
  const poly2 = initPoly2.map((p) => ({ ...p }));
  let numPoints = 200;
  let preserveXScale = true;
  let fixedLength = true;
  let animating = false;
  let animId = 0;
  const dx1 = [0, 0, 0, 0, 0, 0];
  const dy1 = [0, 0, 0, 0, 0, 0];
  const dx2 = [0, 0, 0, 0, 0, 0];
  const dy2 = [0, 0, 0, 0, 0, 0];
  function draw() {
    renderToCanvas({
      demoName: "trans_curve2",
      canvas,
      width: W,
      height: H,
      params: [
        numPoints,
        ...poly1.flatMap((v) => [v.x, v.y]),
        ...poly2.flatMap((v) => [v.x, v.y]),
        preserveXScale ? 1 : 0,
        fixedLength ? 1 : 0,
        animating ? 1 : 0
      ],
      timeDisplay: timeEl
    });
  }
  function movePoint(v, pdx, pdy, i) {
    if (v.x < 0) {
      v.x = 0;
      pdx[i] = -pdx[i];
    }
    if (v.x > W) {
      v.x = W;
      pdx[i] = -pdx[i];
    }
    if (v.y < 0) {
      v.y = 0;
      pdy[i] = -pdy[i];
    }
    if (v.y > H) {
      v.y = H;
      pdy[i] = -pdy[i];
    }
    v.x += pdx[i];
    v.y += pdy[i];
  }
  function normalizePoint(i) {
    const ddx = poly2[i].x - poly1[i].x;
    const ddy = poly2[i].y - poly1[i].y;
    const d = Math.sqrt(ddx * ddx + ddy * ddy);
    if (d > 28.28) {
      poly2[i].x = poly1[i].x + ddx * 28.28 / d;
      poly2[i].y = poly1[i].y + ddy * 28.28 / d;
    }
  }
  function animateFrame() {
    for (let i = 0;i < 6; i++) {
      movePoint(poly1[i], dx1, dy1, i);
      movePoint(poly2[i], dx2, dy2, i);
      normalizePoint(i);
    }
    draw();
    if (animating)
      animId = requestAnimationFrame(animateFrame);
  }
  function startAnimation(v) {
    animating = v;
    if (v) {
      for (let i = 0;i < 6; i++) {
        poly1[i].x = initPoly1[i].x;
        poly1[i].y = initPoly1[i].y;
        poly2[i].x = initPoly2[i].x;
        poly2[i].y = initPoly2[i].y;
        dx1[i] = (Math.random() * 1000 - 500) * 0.01;
        dy1[i] = (Math.random() * 1000 - 500) * 0.01;
        dx2[i] = (Math.random() * 1000 - 500) * 0.01;
        dy2[i] = (Math.random() * 1000 - 500) * 0.01;
      }
      animId = requestAnimationFrame(animateFrame);
    } else {
      cancelAnimationFrame(animId);
      draw();
    }
  }
  const allVertices = [...poly1, ...poly2];
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices: allVertices,
    threshold: 10,
    onDrag: draw
  });
  const slPoints = addSlider(sidebar, "Num Points", 10, 400, 200, 10, (v) => {
    numPoints = v;
    draw();
  });
  const cbFixed = addCheckbox(sidebar, "Fixed Length", fixedLength, (v) => {
    fixedLength = v;
    draw();
  });
  const cbPreserve = addCheckbox(sidebar, "Preserve X scale", preserveXScale, (v) => {
    preserveXScale = v;
    draw();
  });
  const cbAnimate = addCheckbox(sidebar, "Animate", false, (v) => startAnimation(v));
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 340, y2: 15, min: 10, max: 400, sidebarEl: slPoints, onChange: (v) => {
      numPoints = v;
      draw();
    } },
    { type: "checkbox", x1: 350, y1: 5, x2: 460, y2: 19, sidebarEl: cbFixed, onChange: (v) => {
      fixedLength = v;
      draw();
    } },
    { type: "checkbox", x1: 465, y1: 5, x2: 595, y2: 19, sidebarEl: cbPreserve, onChange: (v) => {
      preserveXScale = v;
      draw();
    } },
    { type: "checkbox", x1: 350, y1: 25, x2: 460, y2: 39, sidebarEl: cbAnimate, onChange: (v) => startAnimation(v) }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 12 control points (6 per curve) to reshape both curves.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    animating = false;
    cancelAnimationFrame(animId);
    cleanupDrag();
    cleanupCC();
  };
}
var init_trans_curve2 = __esm(() => {
  init_render_canvas();
});

// src/demos/lion_lens.ts
var exports_lion_lens = {};
__export(exports_lion_lens, {
  init: () => init41
});
function init41(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Lion Lens", "Magnifying lens effect on the lion using trans_warp_magnifier — matching C++ lion_lens.cpp.");
  const W = 512, H = 400;
  let magn = 3;
  let radius = 70;
  let lensX = W / 2;
  let lensY = H / 2;
  let angle = 0;
  function draw() {
    renderToCanvas({
      demoName: "lion_lens",
      canvas,
      width: W,
      height: H,
      params: [magn, radius, lensX, lensY, angle],
      timeDisplay: timeEl
    });
  }
  const slMagn = addSlider(sidebar, "Magnification", 0.01, 4, 3, 0.01, (v) => {
    magn = v;
    draw();
  });
  const slRadius = addSlider(sidebar, "Radius", 0, 100, 70, 1, (v) => {
    radius = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 150, y2: 12, min: 0.01, max: 4, sidebarEl: slMagn, onChange: (v) => {
      magn = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 150, y2: 27, min: 0, max: 100, sidebarEl: slRadius, onChange: (v) => {
      radius = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const handleMouseMove = (e) => {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    lensX = (e.clientX - rect.left) * scaleX;
    lensY = (e.clientY - rect.top) * scaleY;
    draw();
  };
  canvas.addEventListener("mousemove", handleMouseMove);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Move mouse over the canvas to position the lens.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener("mousemove", handleMouseMove);
  };
}
var init_lion_lens = __esm(() => {
  init_render_canvas();
});

// src/demos/distortions.ts
var exports_distortions = {};
__export(exports_distortions, {
  init: () => init42
});
function init42(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Distortions", "Animated wave/swirl distortions on image and gradient sources — matching C++ distortions.cpp.");
  const W = 620, H = 360;
  let angle = 20;
  let scale = 1;
  let amplitude = 10;
  let period = 1;
  let distType = 0;
  let centerX = 170;
  let centerY = 200;
  let phase = 0;
  let draggingCenter = false;
  let animationId = 0;
  function draw() {
    renderToCanvas({
      demoName: "distortions",
      canvas,
      width: W,
      height: H,
      params: [angle, scale, amplitude, period, distType, centerX, centerY, phase],
      timeDisplay: timeEl
    });
  }
  const slAngle = addSlider(sidebar, "Angle", -180, 180, 20, 1, (v) => {
    angle = v;
    draw();
  });
  const slScale = addSlider(sidebar, "Scale", 0.1, 5, 1, 0.01, (v) => {
    scale = v;
    draw();
  });
  const slAmp = addSlider(sidebar, "Amplitude", 0.1, 40, 10, 0.1, (v) => {
    amplitude = v;
    draw();
  });
  const slPeriod = addSlider(sidebar, "Period", 0.1, 2, 1, 0.01, (v) => {
    period = v;
    draw();
  });
  const radioButtons = addRadioGroup(sidebar, "Distortion Type", ["Wave", "Swirl", "Wave-Swirl", "Swirl-Wave"], distType, (i) => {
    distType = i;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 150, y2: 12, min: -180, max: 180, sidebarEl: slAngle, onChange: (v) => {
      angle = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 150, y2: 27, min: 0.1, max: 5, sidebarEl: slScale, onChange: (v) => {
      scale = v;
      draw();
    } },
    { type: "slider", x1: 175, y1: 5, x2: 320, y2: 12, min: 0.1, max: 2, sidebarEl: slPeriod, onChange: (v) => {
      period = v;
      draw();
    } },
    { type: "slider", x1: 175, y1: 20, x2: 320, y2: 27, min: 0.1, max: 40, sidebarEl: slAmp, onChange: (v) => {
      amplitude = v;
      draw();
    } },
    { type: "radio", x1: 480, y1: 5, x2: 600, y2: 90, numItems: 4, sidebarEls: radioButtons, onChange: (i) => {
      distType = i;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  function eventToAgg(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const yTop = (e.clientY - rect.top) * scaleY;
    return { x, y: H - yTop };
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    draggingCenter = true;
    canvas.setPointerCapture(e.pointerId);
    const p = eventToAgg(e);
    centerX = p.x;
    centerY = p.y;
    draw();
  }
  function onPointerMove(e) {
    if (!draggingCenter)
      return;
    const p = eventToAgg(e);
    centerX = p.x;
    centerY = p.y;
    draw();
  }
  function onPointerUp(e) {
    if (!draggingCenter)
      return;
    draggingCenter = false;
    if (canvas.hasPointerCapture(e.pointerId)) {
      canvas.releasePointerCapture(e.pointerId);
    }
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  function tick() {
    phase += 15 * Math.PI / 180;
    if (phase > Math.PI * 200)
      phase -= Math.PI * 200;
    draw();
    animationId = requestAnimationFrame(tick);
  }
  draw();
  animationId = requestAnimationFrame(tick);
  return () => {
    cancelAnimationFrame(animationId);
    cleanupCC();
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
var init_distortions = __esm(() => {
  init_render_canvas();
});

// src/demos/blend_color.ts
var exports_blend_color = {};
__export(exports_blend_color, {
  init: () => init43
});
function init43(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Blend Color", "Shape with blurred shadow — demonstrates blur compositing matching C++ blend_color.cpp.");
  const W = 440, H = 330;
  let blurRadius = 15;
  function draw() {
    renderToCanvas({
      demoName: "blend_color",
      canvas,
      width: W,
      height: H,
      params: [blurRadius, 10, 10],
      timeDisplay: timeEl
    });
  }
  const slBlur = addSlider(sidebar, "Blur Radius", 0, 40, 15, 0.5, (v) => {
    blurRadius = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 435, y2: 12, min: 0, max: 40, sidebarEl: slBlur, onChange: (v) => {
      blurRadius = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_blend_color = __esm(() => {
  init_render_canvas();
});

// src/demos/component_rendering.ts
var exports_component_rendering = {};
__export(exports_component_rendering, {
  init: () => init44
});
function init44(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Component Rendering", "Three overlapping circles rendered to separate R/G/B gray channels — matching C++ component_rendering.cpp.");
  const W = 440, H = 330;
  let alpha = 255;
  function draw() {
    renderToCanvas({
      demoName: "component_rendering",
      canvas,
      width: W,
      height: H,
      params: [alpha],
      timeDisplay: timeEl
    });
  }
  const slAlpha = addSlider(sidebar, "Alpha", 0, 255, 255, 1, (v) => {
    alpha = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 435, y2: 12, min: 0, max: 255, sidebarEl: slAlpha, onChange: (v) => {
      alpha = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_component_rendering = __esm(() => {
  init_render_canvas();
});

// src/demos/polymorphic_renderer.ts
var exports_polymorphic_renderer = {};
__export(exports_polymorphic_renderer, {
  init: () => init45
});
function init45(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Polymorphic Renderer", "Same shapes rendered with different pixel formats (RGBA32, RGB24, Gray8) — matching C++ polymorphic_renderer.cpp.");
  const W = 400, H = 330;
  let format = 0;
  function draw() {
    renderToCanvas({
      demoName: "polymorphic_renderer",
      canvas,
      width: W,
      height: H,
      params: [format],
      timeDisplay: timeEl
    });
  }
  const radioDiv = document.createElement("div");
  radioDiv.className = "control-group";
  const radioLabel = document.createElement("label");
  radioLabel.className = "control-label";
  radioLabel.textContent = "Pixel Format";
  radioDiv.appendChild(radioLabel);
  const names = ["RGBA32 (4 bpp)", "RGB24 (3 bpp)", "Gray8 (1 bpp)"];
  names.forEach((name, i) => {
    const row = document.createElement("label");
    row.style.display = "block";
    row.style.cursor = "pointer";
    row.style.marginBottom = "2px";
    const rb = document.createElement("input");
    rb.type = "radio";
    rb.name = "poly_format";
    rb.value = String(i);
    rb.checked = i === format;
    rb.addEventListener("change", () => {
      format = i;
      draw();
    });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(" " + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Select a pixel format to see the same triangle & circle rendered differently.";
  sidebar.appendChild(hint);
  draw();
}
var init_polymorphic_renderer = __esm(() => {
  init_render_canvas();
});

// src/demos/scanline_boolean.ts
var exports_scanline_boolean = {};
__export(exports_scanline_boolean, {
  init: () => init46
});
function init46(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Scanline Boolean", "Two overlapping circle groups combined with boolean operations — matching C++ scanline_boolean.cpp.");
  const W = 800;
  const H = 600;
  const defaultQuad1 = () => [
    { x: 50, y: 180 },
    { x: W / 2 - 25, y: 200 },
    { x: W / 2 - 25, y: H - 70 },
    { x: 50, y: H - 50 }
  ];
  const defaultQuad2 = () => [
    { x: W / 2 + 25, y: 180 },
    { x: W - 50, y: 200 },
    { x: W - 50, y: H - 70 },
    { x: W / 2 + 25, y: H - 50 }
  ];
  let operation = 0;
  let opacity1 = 1;
  let opacity2 = 1;
  let quad1 = defaultQuad1();
  let quad2 = defaultQuad2();
  function packedQuad(q) {
    return [q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y];
  }
  function draw() {
    renderToCanvas({
      demoName: "scanline_boolean",
      canvas,
      width: W,
      height: H,
      params: [
        operation,
        opacity1,
        opacity2,
        ...packedQuad(quad1),
        ...packedQuad(quad2)
      ],
      timeDisplay: timeEl
    });
  }
  const opRadios = addRadioGroup(sidebar, "Operation", [
    "Union",
    "Intersection",
    "Linear XOR",
    "Saddle XOR",
    "Abs Diff XOR",
    "A-B",
    "B-A"
  ], operation, (i) => {
    operation = i;
    draw();
  });
  const opacity1Slider = addSlider(sidebar, "Opacity1", 0, 1, opacity1, 0.001, (v) => {
    opacity1 = v;
    draw();
  });
  const opacity2Slider = addSlider(sidebar, "Opacity2", 0, 1, opacity2, 0.001, (v) => {
    opacity2 = v;
    draw();
  });
  const resetCheckbox = addCheckbox(sidebar, "Reset", false, (checked) => {
    if (!checked)
      return;
    quad1 = defaultQuad1();
    quad2 = defaultQuad2();
    resetCheckbox.checked = false;
    draw();
  });
  const canvasControls = [
    {
      type: "slider",
      x1: 5,
      y1: 5,
      x2: 340,
      y2: 12,
      min: 0,
      max: 1,
      sidebarEl: opacity1Slider,
      onChange: (v) => {
        opacity1 = v;
        draw();
      }
    },
    {
      type: "slider",
      x1: 5,
      y1: 20,
      x2: 340,
      y2: 27,
      min: 0,
      max: 1,
      sidebarEl: opacity2Slider,
      onChange: (v) => {
        opacity2 = v;
        draw();
      }
    },
    {
      type: "checkbox",
      x1: 350,
      y1: 5,
      x2: 410,
      y2: 20,
      sidebarEl: resetCheckbox,
      onChange: () => {}
    },
    {
      type: "radio",
      x1: 420,
      y1: 5,
      x2: 550,
      y2: 145,
      numItems: 7,
      sidebarEls: opRadios,
      onChange: (i) => {
        operation = i;
        draw();
      }
    }
  ];
  const cleanupCanvasControls = setupCanvasControls(canvas, canvasControls, draw);
  function aggPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY
    };
  }
  function pointInPoly(p, poly) {
    let inside = false;
    let j = poly.length - 1;
    for (let i = 0;i < poly.length; i++) {
      const pi = poly[i];
      const pj = poly[j];
      const intersects = pi.y > p.y !== pj.y > p.y && p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y || 0.000000000001) + pi.x;
      if (intersects)
        inside = !inside;
      j = i;
    }
    return inside;
  }
  function inControlBounds(p) {
    for (const c of canvasControls) {
      const extra = c.type === "slider" || c.type === "scale" ? (c.y2 - c.y1) / 2 : 0;
      if (p.x >= c.x1 - extra && p.x <= c.x2 + extra && p.y >= c.y1 - extra && p.y <= c.y2 + extra) {
        return true;
      }
    }
    return false;
  }
  let drag = null;
  function nearestVertex(p) {
    let best = null;
    const threshold = 10;
    const candidates = [
      { quad: 1, poly: quad1 },
      { quad: 2, poly: quad2 }
    ];
    for (const c of candidates) {
      for (let i = 0;i < c.poly.length; i++) {
        const v = c.poly[i];
        const d = Math.hypot(p.x - v.x, p.y - v.y);
        if (d <= threshold && (!best || d < best.d)) {
          best = { quad: c.quad, idx: i, d };
        }
      }
    }
    return best;
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const p = aggPos2(e);
    if (inControlBounds(p))
      return;
    const near = nearestVertex(p);
    if (near) {
      const poly = near.quad === 1 ? quad1 : quad2;
      drag = {
        kind: "vertex",
        quad: near.quad,
        idx: near.idx,
        dx: p.x - poly[near.idx].x,
        dy: p.y - poly[near.idx].y
      };
      canvas.setPointerCapture(e.pointerId);
      return;
    }
    if (pointInPoly(p, quad1)) {
      drag = { kind: "all", quad: 1, lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
      return;
    }
    if (pointInPoly(p, quad2)) {
      drag = { kind: "all", quad: 2, lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
    }
  }
  function onPointerMove(e) {
    if (!drag)
      return;
    const p = aggPos2(e);
    if (drag.kind === "vertex") {
      const poly = drag.quad === 1 ? quad1 : quad2;
      poly[drag.idx].x = p.x - drag.dx;
      poly[drag.idx].y = p.y - drag.dy;
    } else {
      const poly = drag.quad === 1 ? quad1 : quad2;
      const ddx = p.x - drag.lastX;
      const ddy = p.y - drag.lastY;
      for (const v of poly) {
        v.x += ddx;
        v.y += ddy;
      }
      drag.lastX = p.x;
      drag.lastY = p.y;
    }
    draw();
  }
  function onPointerUp() {
    drag = null;
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  draw();
  return () => {
    cleanupCanvasControls();
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
var init_scanline_boolean = __esm(() => {
  init_render_canvas();
});

// src/demos/scanline_boolean2.ts
var exports_scanline_boolean2 = {};
__export(exports_scanline_boolean2, {
  init: () => init47
});
function init47(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Scanline Boolean 2", "Boolean operations on complex shapes — matching C++ scanline_boolean2.cpp.");
  const W = 655, H = 520;
  let polygonIdx = 3;
  let fillRuleIdx = 1;
  let scanlineIdx = 1;
  let operationIdx = 2;
  let mouseX = W / 2;
  let mouseY = H / 2;
  let isDragging = false;
  function draw() {
    renderToCanvas({
      demoName: "scanline_boolean2",
      canvas,
      width: W,
      height: H,
      params: [polygonIdx, fillRuleIdx, scanlineIdx, operationIdx, mouseX, mouseY],
      timeDisplay: timeEl
    });
  }
  const polyRadios = addRadioGroup(sidebar, "Polygons", [
    "Two Simple Paths",
    "Closed Stroke",
    "Great Britain and Arrows",
    "Great Britain and Spiral",
    "Spiral and Glyph"
  ], polygonIdx, (i) => {
    polygonIdx = i;
    draw();
  });
  const fillRadios = addRadioGroup(sidebar, "Fill Rule", [
    "Even-Odd",
    "Non Zero"
  ], fillRuleIdx, (i) => {
    fillRuleIdx = i;
    draw();
  });
  const slRadios = addRadioGroup(sidebar, "Scanline Type", [
    "scanline_p",
    "scanline_u",
    "scanline_bin"
  ], scanlineIdx, (i) => {
    scanlineIdx = i;
    draw();
  });
  const opRadios = addRadioGroup(sidebar, "Operation", [
    "None",
    "OR",
    "AND",
    "XOR Linear",
    "XOR Saddle",
    "A-B",
    "B-A"
  ], operationIdx, (i) => {
    operationIdx = i;
    draw();
  });
  const canvasControls = [
    {
      type: "radio",
      x1: 5,
      y1: 5,
      x2: 210,
      y2: 110,
      numItems: 5,
      sidebarEls: polyRadios,
      onChange: (i) => {
        polygonIdx = i;
        draw();
      }
    },
    {
      type: "radio",
      x1: 200,
      y1: 5,
      x2: 305,
      y2: 50,
      numItems: 2,
      sidebarEls: fillRadios,
      onChange: (i) => {
        fillRuleIdx = i;
        draw();
      }
    },
    {
      type: "radio",
      x1: 300,
      y1: 5,
      x2: 415,
      y2: 70,
      numItems: 3,
      sidebarEls: slRadios,
      onChange: (i) => {
        scanlineIdx = i;
        draw();
      }
    },
    {
      type: "radio",
      x1: 535,
      y1: 5,
      x2: 650,
      y2: 145,
      numItems: 7,
      sidebarEls: opRadios,
      onChange: (i) => {
        operationIdx = i;
        draw();
      }
    }
  ];
  setupCanvasControls(canvas, canvasControls, draw);
  function aggPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY
    };
  }
  canvas.addEventListener("pointerdown", (e) => {
    if (e.button !== 0)
      return;
    const pos = aggPos2(e);
    for (const c of canvasControls) {
      if (pos.x >= c.x1 && pos.x <= c.x2 && pos.y >= c.y1 && pos.y <= c.y2) {
        return;
      }
    }
    isDragging = true;
    mouseX = pos.x;
    mouseY = pos.y;
    canvas.setPointerCapture(e.pointerId);
    draw();
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!isDragging)
      return;
    const pos = aggPos2(e);
    mouseX = pos.x;
    mouseY = pos.y;
    draw();
  });
  canvas.addEventListener("pointerup", () => {
    isDragging = false;
  });
  canvas.addEventListener("pointercancel", () => {
    isDragging = false;
  });
  draw();
}
var init_scanline_boolean2 = __esm(() => {
  init_render_canvas();
});

// src/demos/pattern_fill.ts
var exports_pattern_fill = {};
__export(exports_pattern_fill, {
  init: () => init48
});
function init48(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Pattern Fill", "Star polygon filled with a repeating tile pattern — matching C++ pattern_fill.cpp.");
  const W = 512, H = 400;
  let patSize = 30;
  let polyAngle = 0;
  function draw() {
    renderToCanvas({
      demoName: "pattern_fill",
      canvas,
      width: W,
      height: H,
      params: [patSize, polyAngle],
      timeDisplay: timeEl
    });
  }
  const slSize = addSlider(sidebar, "Pattern Size", 10, 60, 30, 1, (v) => {
    patSize = v;
    draw();
  });
  const slAngle = addSlider(sidebar, "Polygon Angle", -180, 180, 0, 1, (v) => {
    polyAngle = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 507, y2: 12, min: 10, max: 60, sidebarEl: slSize, onChange: (v) => {
      patSize = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 507, y2: 27, min: -180, max: 180, sidebarEl: slAngle, onChange: (v) => {
      polyAngle = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  return cleanupCC;
}
var init_pattern_fill = __esm(() => {
  init_render_canvas();
});

// src/demos/pattern_perspective.ts
var exports_pattern_perspective = {};
__export(exports_pattern_perspective, {
  init: () => init49
});
function init49(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Pattern Perspective", "Perspective-transformed pattern fill in a draggable quad — matching C++ pattern_perspective.cpp.");
  const W = 600, H = 600;
  const vertices = [
    { x: W * 0.17, y: H * 0.17 },
    { x: W * 0.83, y: H * 0.08 },
    { x: W * 0.83, y: H * 0.83 },
    { x: W * 0.17, y: H * 0.83 }
  ];
  let transType = 0;
  function draw() {
    renderToCanvas({
      demoName: "pattern_perspective",
      canvas,
      width: W,
      height: H,
      params: [
        ...vertices.flatMap((v) => [v.x, v.y]),
        transType
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const radioDiv = document.createElement("div");
  radioDiv.className = "control-group";
  const radioLabel = document.createElement("label");
  radioLabel.className = "control-label";
  radioLabel.textContent = "Transform Type";
  radioDiv.appendChild(radioLabel);
  const names = ["Affine", "Bilinear", "Perspective"];
  names.forEach((name, i) => {
    const row = document.createElement("label");
    row.style.display = "block";
    row.style.cursor = "pointer";
    row.style.marginBottom = "2px";
    const rb = document.createElement("input");
    rb.type = "radio";
    rb.name = "pat_persp_trans";
    rb.value = String(i);
    rb.checked = i === transType;
    rb.addEventListener("change", () => {
      transType = i;
      draw();
    });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(" " + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 4 quad corners to transform the pattern.";
  sidebar.appendChild(hint);
  draw();
  return cleanupDrag;
}
var init_pattern_perspective = __esm(() => {
  init_render_canvas();
});

// src/demos/pattern_resample.ts
var exports_pattern_resample = {};
__export(exports_pattern_resample, {
  init: () => init50
});
function init50(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Pattern Resample", "Perspective-transformed procedural image with gamma control — matching C++ pattern_resample.cpp.");
  const W = 600, H = 600;
  const vertices = [
    { x: W * 0.2, y: H * 0.2 },
    { x: W * 0.8, y: H * 0.15 },
    { x: W * 0.85, y: H * 0.8 },
    { x: W * 0.15, y: H * 0.85 }
  ];
  let gamma = 1;
  function draw() {
    renderToCanvas({
      demoName: "pattern_resample",
      canvas,
      width: W,
      height: H,
      params: [
        ...vertices.flatMap((v) => [v.x, v.y]),
        gamma
      ],
      timeDisplay: timeEl
    });
  }
  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw
  });
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 3, 1, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 595, y2: 12, min: 0.5, max: 3, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Drag the 4 quad corners to transform. Adjust gamma.";
  sidebar.appendChild(hint);
  draw();
  return () => {
    cleanupDrag();
    cleanupCC();
  };
}
var init_pattern_resample = __esm(() => {
  init_render_canvas();
});

// src/demos/lion_outline.ts
var exports_lion_outline = {};
__export(exports_lion_outline, {
  init: () => init51
});
function init51(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Lion Outline", "Lion rendered with anti-aliased outline rasterizer vs scanline rasterizer — matching C++ lion_outline.cpp.");
  const W = 512, H = 512;
  let angle = 0;
  let scale = 1;
  let skewX = 0;
  let skewY = 0;
  let lineWidth = 1;
  let useScanline = 0;
  function draw() {
    renderToCanvas({
      demoName: "lion_outline",
      canvas,
      width: W,
      height: H,
      params: [angle, scale, skewX, skewY, lineWidth, useScanline],
      timeDisplay: timeEl
    });
  }
  const slWidth = addSlider(sidebar, "Width", 0, 4, 1, 0.01, (v) => {
    lineWidth = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 150, y2: 12, min: 0, max: 4, sidebarEl: slWidth, onChange: (v) => {
      lineWidth = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const cbDiv = document.createElement("div");
  cbDiv.className = "control-group";
  const cb = document.createElement("input");
  cb.type = "checkbox";
  cb.id = "lion_outline_scanline";
  cb.checked = false;
  cb.addEventListener("change", () => {
    useScanline = cb.checked ? 1 : 0;
    draw();
  });
  const cbLabel = document.createElement("label");
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = " Use Scanline Rasterizer";
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);
  let dragging = false;
  const onDown = (e) => {
    dragging = true;
    canvas.setPointerCapture(e.pointerId);
    updateTransform(e);
  };
  const onMove = (e) => {
    if (dragging)
      updateTransform(e);
  };
  const onUp = () => {
    dragging = false;
  };
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  canvas.addEventListener("pointercancel", onUp);
  function updateTransform(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx - W / 2;
    const y = (e.clientY - rect.top) * sy - H / 2;
    angle = Math.atan2(y, x);
    scale = Math.sqrt(x * x + y * y) / 100;
    draw();
  }
  draw();
  return cleanupCC;
}
var init_lion_outline = __esm(() => {
  init_render_canvas();
});

// src/demos/rasterizers2.ts
var exports_rasterizers2 = {};
__export(exports_rasterizers2, {
  init: () => init52
});
function init52(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Rasterizers 2", "Comparison of different rasterization techniques: aliased, AA outline, scanline, and image pattern — matching C++ rasterizers2.cpp.");
  const W = 500, H = 450;
  let step = 0.1;
  let lineWidth = 3;
  let accurateJoins = 0;
  let startAngle = 0;
  let scalePattern = 1;
  let rotating = false;
  let perfTesting = false;
  let animId = 0;
  function draw() {
    renderToCanvas({
      demoName: "rasterizers2",
      canvas,
      width: W,
      height: H,
      params: [step, lineWidth, accurateJoins, startAngle, scalePattern, rotating ? 1 : 0, perfTesting ? 1 : 0],
      timeDisplay: timeEl
    });
  }
  function startStop(v) {
    if (perfTesting && v) {
      cbRotate.checked = false;
      return;
    }
    if (v !== rotating) {
      rotating = v;
      draw();
    }
  }
  function tick() {
    if (rotating) {
      startAngle += step;
      if (startAngle > 360)
        startAngle -= 360;
      draw();
    }
    animId = requestAnimationFrame(tick);
  }
  async function runPerformanceTest() {
    if (perfTesting)
      return;
    perfTesting = true;
    const wasRotating = rotating;
    try {
      if (wasRotating)
        startStop(false);
      cbRotate.checked = false;
      cbTest.checked = true;
      draw();
      await new Promise((resolve) => requestAnimationFrame(() => resolve()));
      const iterations = 200;
      let benchAngle = startAngle;
      const t0 = performance.now();
      for (let i = 0;i < iterations; i++) {
        benchAngle += step;
        if (benchAngle > 360)
          benchAngle -= 360;
        renderDemo("rasterizers2", W, H, [step, lineWidth, accurateJoins, benchAngle, scalePattern, 0, 1]);
      }
      const elapsed = performance.now() - t0;
      startAngle = benchAngle;
      draw();
      window.alert(`Rasterizers2 benchmark (${iterations} frames)
` + `Total: ${elapsed.toFixed(2)} ms
` + `Average: ${(elapsed / iterations).toFixed(3)} ms/frame`);
    } finally {
      perfTesting = false;
      cbTest.checked = false;
      draw();
      if (wasRotating) {
        cbRotate.checked = true;
        startStop(true);
      }
    }
  }
  const slStep = addSlider(sidebar, "Step", 0, 2, 0.1, 0.01, (v) => {
    step = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 0, 14, 3, 0.01, (v) => {
    lineWidth = v;
    draw();
  });
  const cbTest = addCheckbox(sidebar, "Test Performance", false, (v) => {
    if (v) {
      runPerformanceTest();
    }
  });
  const cbRotate = addCheckbox(sidebar, "Rotate", false, (v) => startStop(v));
  const cbAccurate = addCheckbox(sidebar, "Accurate Joins", false, (v) => {
    accurateJoins = v ? 1 : 0;
    draw();
  });
  const cbScale = addCheckbox(sidebar, "Scale Pattern", true, (v) => {
    scalePattern = v ? 1 : 0;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 10, y1: 14, x2: 150, y2: 22, min: 0, max: 2, sidebarEl: slStep, onChange: (v) => {
      step = v;
      draw();
    } },
    { type: "slider", x1: 160, y1: 14, x2: 390, y2: 22, min: 0, max: 14, sidebarEl: slWidth, onChange: (v) => {
      lineWidth = v;
      draw();
    } },
    { type: "checkbox", x1: 10, y1: 30, x2: 130, y2: 44, sidebarEl: cbTest, onChange: (v) => {
      if (v)
        runPerformanceTest();
    } },
    { type: "checkbox", x1: 140, y1: 30, x2: 200, y2: 44, sidebarEl: cbRotate, onChange: (v) => startStop(v) },
    { type: "checkbox", x1: 210, y1: 30, x2: 310, y2: 44, sidebarEl: cbAccurate, onChange: (v) => {
      accurateJoins = v ? 1 : 0;
      draw();
    } },
    { type: "checkbox", x1: 320, y1: 30, x2: 420, y2: 44, sidebarEl: cbScale, onChange: (v) => {
      scalePattern = v ? 1 : 0;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  draw();
  animId = requestAnimationFrame(tick);
  return () => {
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
var init_rasterizers2 = __esm(() => {
  init_render_canvas();
});

// src/demos/line_patterns.ts
var exports_line_patterns = {};
__export(exports_line_patterns, {
  init: () => init53
});
function init53(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Line Patterns", "Drawing bezier curves with image patterns — port of C++ line_patterns.cpp. Each curve uses a different procedural pattern sampled along its length.");
  const W = 500, H = 450;
  let scaleX = 1;
  let startX = 0;
  const points = [
    64,
    19,
    14,
    126,
    118,
    266,
    19,
    265,
    112,
    113,
    178,
    32,
    200,
    132,
    125,
    438,
    401,
    24,
    326,
    149,
    285,
    11,
    177,
    77,
    188,
    427,
    129,
    295,
    19,
    283,
    25,
    410,
    451,
    346,
    302,
    218,
    265,
    441,
    459,
    400,
    454,
    198,
    14,
    13,
    220,
    291,
    483,
    283,
    301,
    398,
    355,
    231,
    209,
    211,
    170,
    353,
    484,
    101,
    222,
    33,
    486,
    435,
    487,
    138,
    143,
    147,
    11,
    45,
    83,
    427,
    132,
    197
  ];
  function draw() {
    renderToCanvas({
      demoName: "line_patterns",
      canvas,
      width: W,
      height: H,
      params: [scaleX, startX, ...points],
      timeDisplay: timeEl
    });
  }
  const slScale = addSlider(sidebar, "Scale X", 0.2, 3, 1, 0.01, (v) => {
    scaleX = v;
    draw();
  });
  const slStart = addSlider(sidebar, "Start X", 0, 10, 0, 0.01, (v) => {
    startX = v;
    draw();
  });
  const sliders = [
    { x1: 5, x2: 240, yTop: H - 12, yBot: H - 5, min: 0.2, max: 3, el: slScale, set: (v) => {
      scaleX = v;
    } },
    { x1: 250, x2: 495, yTop: H - 12, yBot: H - 5, min: 0, max: 10, el: slStart, set: (v) => {
      startX = v;
    } }
  ];
  const GRAB_RADIUS = 10;
  let dragMode = null;
  let activeSliderIdx = -1;
  let dragPointIdx = -1;
  function screenPos(e) {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (W / rect.width),
      y: (e.clientY - rect.top) * (H / rect.height)
    };
  }
  function hitSlider(x, y) {
    for (let i = 0;i < sliders.length; i++) {
      const s = sliders[i];
      const extra = (s.yBot - s.yTop) / 2;
      if (x >= s.x1 - extra && x <= s.x2 + extra && y >= s.yTop - extra && y <= s.yBot + extra) {
        return i;
      }
    }
    return -1;
  }
  function sliderValue(idx, x) {
    const s = sliders[idx];
    const xs1 = s.x1 + 1;
    const xs2 = s.x2 - 1;
    let t = (x - xs1) / (xs2 - xs1);
    t = Math.max(0, Math.min(1, t));
    return s.min + t * (s.max - s.min);
  }
  function findPoint(mx, my) {
    let bestD2 = GRAB_RADIUS * GRAB_RADIUS;
    let bestIdx = -1;
    for (let i = 0;i < points.length; i += 2) {
      const dx = points[i] - mx;
      const dy = points[i + 1] - my;
      const d2 = dx * dx + dy * dy;
      if (d2 < bestD2) {
        bestD2 = d2;
        bestIdx = i;
      }
    }
    return bestIdx;
  }
  function onDown(e) {
    if (e.button !== 0)
      return;
    const p = screenPos(e);
    const si = hitSlider(p.x, p.y);
    if (si >= 0) {
      dragMode = "slider";
      activeSliderIdx = si;
      canvas.setPointerCapture(e.pointerId);
      const v = sliderValue(si, p.x);
      sliders[si].set(v);
      sliders[si].el.value = String(v);
      sliders[si].el.dispatchEvent(new Event("input"));
      e.preventDefault();
      return;
    }
    const pi = findPoint(p.x, p.y);
    if (pi >= 0) {
      dragMode = "point";
      dragPointIdx = pi;
      canvas.setPointerCapture(e.pointerId);
      e.preventDefault();
      return;
    }
  }
  function onMove(e) {
    if (!dragMode)
      return;
    const p = screenPos(e);
    if (dragMode === "slider") {
      const v = sliderValue(activeSliderIdx, p.x);
      sliders[activeSliderIdx].set(v);
      sliders[activeSliderIdx].el.value = String(v);
      sliders[activeSliderIdx].el.dispatchEvent(new Event("input"));
    } else if (dragMode === "point") {
      points[dragPointIdx] = p.x;
      points[dragPointIdx + 1] = p.y;
      draw();
    }
    e.preventDefault();
  }
  function onUp() {
    dragMode = null;
    activeSliderIdx = -1;
    dragPointIdx = -1;
  }
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  canvas.addEventListener("pointercancel", onUp);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onDown);
    canvas.removeEventListener("pointermove", onMove);
    canvas.removeEventListener("pointerup", onUp);
    canvas.removeEventListener("pointercancel", onUp);
  };
}
var init_line_patterns = __esm(() => {
  init_render_canvas();
});

// src/demos/line_patterns_clip.ts
var exports_line_patterns_clip = {};
__export(exports_line_patterns_clip, {
  init: () => init54
});
function init54(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Line Patterns Clip", "Anti-aliased outline spirals clipped to a region — adapted from C++ line_patterns_clip.cpp.");
  const W = 500, H = 450;
  let lineWidth = 3;
  let accurateJoins = 0;
  let startAngle = 0;
  function draw() {
    renderToCanvas({
      demoName: "line_patterns_clip",
      canvas,
      width: W,
      height: H,
      params: [lineWidth, accurateJoins, startAngle],
      timeDisplay: timeEl
    });
  }
  const slWidth = addSlider(sidebar, "Width", 0.5, 10, 3, 0.01, (v) => {
    lineWidth = v;
    draw();
  });
  addSlider(sidebar, "Start Angle", 0, 360, 0, 1, (v) => {
    startAngle = v;
    draw();
  });
  const canvasControls = [
    { type: "slider", x1: 10, y1: 14, x2: 490, y2: 22, min: 0.5, max: 10, sidebarEl: slWidth, onChange: (v) => {
      lineWidth = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);
  const cbDiv = document.createElement("div");
  cbDiv.className = "control-group";
  const cb = document.createElement("input");
  cb.type = "checkbox";
  cb.id = "lpc_accurate";
  cb.checked = false;
  cb.addEventListener("change", () => {
    accurateJoins = cb.checked ? 1 : 0;
    draw();
  });
  const cbLabel = document.createElement("label");
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = " Accurate Joins";
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);
  draw();
  return cleanupCC;
}
var init_line_patterns_clip = __esm(() => {
  init_render_canvas();
});

// src/demos/compositing.ts
var exports_compositing = {};
__export(exports_compositing, {
  init: () => init55
});
function init55(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Compositing", "SVG compositing operations — two shapes blended with selectable comp_op mode.");
  const W = 600, H = 400;
  let compOp = 3, srcAlpha = 0.75, dstAlpha = 1;
  function draw() {
    renderToCanvas({
      demoName: "compositing",
      canvas,
      width: W,
      height: H,
      params: [compOp, srcAlpha, dstAlpha],
      timeDisplay: timeEl
    });
  }
  const slSrc = addSlider(sidebar, "Src Alpha", 0, 1, 0.75, 0.01, (v) => {
    srcAlpha = v;
    draw();
  });
  const slDst = addSlider(sidebar, "Dst Alpha", 0, 1, 1, 0.01, (v) => {
    dstAlpha = v;
    draw();
  });
  const displayCompOps = [...COMP_OP_NAMES].reverse();
  const initialDisplayIndex = COMP_OP_NAMES.length - 1 - compOp;
  const radiosDisplay = addRadioGroup(sidebar, "Comp Op", displayCompOps, initialDisplayIndex, (i) => {
    compOp = COMP_OP_NAMES.length - 1 - i;
    draw();
  });
  const radiosByCompOp = new Array(COMP_OP_NAMES.length);
  for (let displayIdx = 0;displayIdx < radiosDisplay.length; displayIdx++) {
    const compOpIdx = COMP_OP_NAMES.length - 1 - displayIdx;
    radiosByCompOp[compOpIdx] = radiosDisplay[displayIdx];
  }
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 400, y2: 11, min: 0, max: 1, sidebarEl: slSrc, onChange: (v) => {
      srcAlpha = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 400, y2: 26, min: 0, max: 1, sidebarEl: slDst, onChange: (v) => {
      dstAlpha = v;
      draw();
    } },
    { type: "radio", x1: 420, y1: 5, x2: 590, y2: 340, numItems: COMP_OP_NAMES.length, sidebarEls: radiosByCompOp, onChange: (i) => {
      compOp = i;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  draw();
  return () => cleanupCC();
}
var COMP_OP_NAMES;
var init_compositing = __esm(() => {
  init_render_canvas();
  COMP_OP_NAMES = [
    "clear",
    "src",
    "dst",
    "src-over",
    "dst-over",
    "src-in",
    "dst-in",
    "src-out",
    "dst-out",
    "src-atop",
    "dst-atop",
    "xor",
    "plus",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion"
  ];
});

// src/demos/compositing2.ts
var exports_compositing2 = {};
__export(exports_compositing2, {
  init: () => init56
});
function init56(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Compositing 2", "Multiple overlapping circles blended with selected SVG compositing mode.");
  const W = 600, H = 400;
  let compOp = 3, srcAlpha = 1, dstAlpha = 1;
  function draw() {
    renderToCanvas({
      demoName: "compositing2",
      canvas,
      width: W,
      height: H,
      params: [compOp, srcAlpha, dstAlpha],
      timeDisplay: timeEl
    });
  }
  const slDst = addSlider(sidebar, "Dst Alpha", 0, 1, 1, 0.01, (v) => {
    dstAlpha = v;
    draw();
  });
  const slSrc = addSlider(sidebar, "Src Alpha", 0, 1, 1, 0.01, (v) => {
    srcAlpha = v;
    draw();
  });
  const displayCompOps = [...COMP_OP_NAMES2].reverse();
  const initialDisplayIndex = COMP_OP_NAMES2.length - 1 - compOp;
  const radiosDisplay = addRadioGroup(sidebar, "Comp Op", displayCompOps, initialDisplayIndex, (i) => {
    compOp = COMP_OP_NAMES2.length - 1 - i;
    draw();
  });
  const radiosByCompOp = new Array(COMP_OP_NAMES2.length);
  for (let displayIdx = 0;displayIdx < radiosDisplay.length; displayIdx++) {
    const compOpIdx = COMP_OP_NAMES2.length - 1 - displayIdx;
    radiosByCompOp[compOpIdx] = radiosDisplay[displayIdx];
  }
  const canvasControls = [
    { type: "slider", x1: 5, y1: 5, x2: 400, y2: 11, min: 0, max: 1, sidebarEl: slDst, onChange: (v) => {
      dstAlpha = v;
      draw();
    } },
    { type: "slider", x1: 5, y1: 20, x2: 400, y2: 26, min: 0, max: 1, sidebarEl: slSrc, onChange: (v) => {
      srcAlpha = v;
      draw();
    } },
    { type: "radio", x1: 420, y1: 5, x2: 590, y2: 340, numItems: COMP_OP_NAMES2.length, sidebarEls: radiosByCompOp, onChange: (i) => {
      compOp = i;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  draw();
  return () => cleanupCC();
}
var COMP_OP_NAMES2;
var init_compositing2 = __esm(() => {
  init_render_canvas();
  COMP_OP_NAMES2 = [
    "clear",
    "src",
    "dst",
    "src-over",
    "dst-over",
    "src-in",
    "dst-in",
    "src-out",
    "dst-out",
    "src-atop",
    "dst-atop",
    "xor",
    "plus",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion"
  ];
});

// src/demos/flash_rasterizer.ts
var exports_flash_rasterizer = {};
__export(exports_flash_rasterizer, {
  init: () => init57
});
function init57(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Flash Rasterizer", "Compound rasterizer with multi-style filled shapes — matching C++ flash_rasterizer.cpp behavior.");
  const W = 655, H = 520;
  const m = [1, 0, 0, 1, 0, 0];
  let shapeIndex = 0;
  const editedVertices = new Map;
  let pointerX = W * 0.5;
  let pointerY = H * 0.5;
  let dragVertex = -1;
  let rightMouseHeld = false;
  let dragUsesFlippedY = false;
  function mulInPlace(b) {
    const a0 = m[0], a1 = m[1], a2 = m[2], a3 = m[3], a4 = m[4], a5 = m[5];
    m[0] = a0 * b[0] + a1 * b[2];
    m[2] = a2 * b[0] + a3 * b[2];
    m[4] = a4 * b[0] + a5 * b[2] + b[4];
    m[1] = a0 * b[1] + a1 * b[3];
    m[3] = a2 * b[1] + a3 * b[3];
    m[5] = a4 * b[1] + a5 * b[3] + b[5];
  }
  function translate(tx, ty) {
    return [1, 0, 0, 1, tx, ty];
  }
  function scale(s) {
    return [s, 0, 0, s, 0, 0];
  }
  function rotate(a) {
    const c = Math.cos(a);
    const s = Math.sin(a);
    return [c, s, -s, c, 0, 0];
  }
  function applyAroundPointer(op) {
    mulInPlace(translate(-pointerX, -pointerY));
    mulInPlace(op);
    mulInPlace(translate(pointerX, pointerY));
  }
  function approxScale() {
    return Math.max(0.001, Math.hypot(m[0], m[2]));
  }
  function buildParams() {
    const edits = [];
    const sorted = [...editedVertices.entries()].sort((a, b) => a[0] - b[0]);
    for (const [idx, p] of sorted)
      edits.push(idx, p.x, p.y);
    return [
      shapeIndex,
      m[0],
      m[1],
      m[2],
      m[3],
      m[4],
      m[5],
      pointerX,
      pointerY,
      rightMouseHeld ? 1 : 0,
      ...edits
    ];
  }
  function draw() {
    renderToCanvas({
      demoName: "flash_rasterizer",
      canvas,
      width: W,
      height: H,
      params: buildParams(),
      timeDisplay: timeEl,
      flipY: false
    });
    shapeInfo.textContent = `Shape: ${shapeIndex}`;
  }
  function addButton(label, onClick) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = label;
    btn.style.cssText = "display:block;margin:4px 0;padding:6px 10px;cursor:pointer;font-size:12px;width:100%;";
    btn.addEventListener("click", onClick);
    sidebar.appendChild(btn);
    return btn;
  }
  const shapeInfo = document.createElement("div");
  shapeInfo.className = "control-hint";
  shapeInfo.textContent = "Shape: 0";
  sidebar.appendChild(shapeInfo);
  addButton("Next Shape (Space)", () => {
    shapeIndex += 1;
    editedVertices.clear();
    dragVertex = -1;
    draw();
  });
  addButton("Zoom In (+)", () => {
    applyAroundPointer(scale(1.1));
    draw();
  });
  addButton("Zoom Out (-)", () => {
    applyAroundPointer(scale(1 / 1.1));
    draw();
  });
  addButton("Rotate Left (←)", () => {
    applyAroundPointer(rotate(-Math.PI / 20));
    draw();
  });
  addButton("Rotate Right (→)", () => {
    applyAroundPointer(rotate(Math.PI / 20));
    draw();
  });
  addButton("Reset View", () => {
    m[0] = 1;
    m[1] = 0;
    m[2] = 0;
    m[3] = 1;
    m[4] = 0;
    m[5] = 0;
    draw();
  });
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Canvas: left-drag vertices, right-hold to test fill hit, arrows rotate around mouse, +/- zoom around mouse.";
  sidebar.appendChild(hint);
  function canvasPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: (e.clientY - rect.top) * sy
    };
  }
  function onKeyDown(e) {
    if (e.key === " ") {
      shapeIndex += 1;
      editedVertices.clear();
      dragVertex = -1;
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "+" || e.key === "=" || e.code === "NumpadAdd") {
      applyAroundPointer(scale(1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "-" || e.code === "NumpadSubtract") {
      applyAroundPointer(scale(1 / 1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "ArrowLeft") {
      applyAroundPointer(rotate(-Math.PI / 20));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "ArrowRight") {
      applyAroundPointer(rotate(Math.PI / 20));
      draw();
      e.preventDefault();
    }
  }
  function onPointerDown(e) {
    const p = canvasPos2(e);
    pointerX = p.x;
    pointerY = p.y;
    if (e.button === 2) {
      rightMouseHeld = true;
      canvas.setPointerCapture(e.pointerId);
      draw();
      e.preventDefault();
      return;
    }
    if (e.button !== 0)
      return;
    const pickRadius = 10 / approxScale();
    let hit = flashPickVertex("flash_rasterizer", W, H, buildParams(), p.x, p.y, pickRadius);
    dragUsesFlippedY = false;
    if (hit < 0) {
      const yFlip = H - p.y;
      hit = flashPickVertex("flash_rasterizer", W, H, buildParams(), p.x, yFlip, pickRadius);
      if (hit >= 0) {
        dragUsesFlippedY = true;
      }
    }
    dragVertex = hit;
    canvas.setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onPointerMove(e) {
    const p = canvasPos2(e);
    pointerX = p.x;
    pointerY = p.y;
    if (dragVertex >= 0 && (e.buttons & 1) !== 0) {
      const dragY = dragUsesFlippedY ? H - p.y : p.y;
      const [lx, ly] = flashScreenToShape("flash_rasterizer", W, H, buildParams(), p.x, dragY);
      editedVertices.set(dragVertex, { x: lx, y: ly });
      draw();
      e.preventDefault();
      return;
    }
    if (rightMouseHeld && (e.buttons & 2) === 0) {
      rightMouseHeld = false;
      draw();
      return;
    }
    if (dragVertex >= 0 && (e.buttons & 1) === 0) {
      dragVertex = -1;
      draw();
    }
  }
  function onPointerUp() {
    const hadRight = rightMouseHeld;
    rightMouseHeld = false;
    dragUsesFlippedY = false;
    if (dragVertex >= 0) {
      dragVertex = -1;
      draw();
      return;
    }
    if (hadRight)
      draw();
  }
  function onContextMenu(e) {
    e.preventDefault();
  }
  window.addEventListener("keydown", onKeyDown);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("contextmenu", onContextMenu);
  draw();
  return () => {
    window.removeEventListener("keydown", onKeyDown);
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    canvas.removeEventListener("contextmenu", onContextMenu);
  };
}
var init_flash_rasterizer = __esm(() => {
  init_render_canvas();
});

// src/demos/flash_rasterizer2.ts
var exports_flash_rasterizer2 = {};
__export(exports_flash_rasterizer2, {
  init: () => init58
});
function init58(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Flash Rasterizer 2", "Multi-style shapes rendered with regular rasterizer — matching C++ flash_rasterizer2.cpp behavior.");
  const W = 655, H = 520;
  const m = [1, 0, 0, 1, 0, 0];
  let shapeIndex = 0;
  const editedVertices = new Map;
  let pointerX = W * 0.5;
  let pointerY = H * 0.5;
  let dragVertex = -1;
  let dragUsesFlippedY = false;
  function mulInPlace(b) {
    const a0 = m[0], a1 = m[1], a2 = m[2], a3 = m[3], a4 = m[4], a5 = m[5];
    m[0] = a0 * b[0] + a1 * b[2];
    m[2] = a2 * b[0] + a3 * b[2];
    m[4] = a4 * b[0] + a5 * b[2] + b[4];
    m[1] = a0 * b[1] + a1 * b[3];
    m[3] = a2 * b[1] + a3 * b[3];
    m[5] = a4 * b[1] + a5 * b[3] + b[5];
  }
  function translate(tx, ty) {
    return [1, 0, 0, 1, tx, ty];
  }
  function scale(s) {
    return [s, 0, 0, s, 0, 0];
  }
  function rotate(a) {
    const c = Math.cos(a);
    const s = Math.sin(a);
    return [c, s, -s, c, 0, 0];
  }
  function applyAroundPointer(op) {
    mulInPlace(translate(-pointerX, -pointerY));
    mulInPlace(op);
    mulInPlace(translate(pointerX, pointerY));
  }
  function approxScale() {
    return Math.max(0.001, Math.hypot(m[0], m[2]));
  }
  function buildParams() {
    const edits = [];
    const sorted = [...editedVertices.entries()].sort((a, b) => a[0] - b[0]);
    for (const [idx, p] of sorted)
      edits.push(idx, p.x, p.y);
    return [
      shapeIndex,
      m[0],
      m[1],
      m[2],
      m[3],
      m[4],
      m[5],
      pointerX,
      pointerY,
      0,
      ...edits
    ];
  }
  function draw() {
    renderToCanvas({
      demoName: "flash_rasterizer2",
      canvas,
      width: W,
      height: H,
      params: buildParams(),
      timeDisplay: timeEl,
      flipY: false
    });
    shapeInfo.textContent = `Shape: ${shapeIndex}`;
  }
  function addButton(label, onClick) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = label;
    btn.style.cssText = "display:block;margin:4px 0;padding:6px 10px;cursor:pointer;font-size:12px;width:100%;";
    btn.addEventListener("click", onClick);
    sidebar.appendChild(btn);
    return btn;
  }
  const shapeInfo = document.createElement("div");
  shapeInfo.className = "control-hint";
  shapeInfo.textContent = "Shape: 0";
  sidebar.appendChild(shapeInfo);
  addButton("Next Shape (Space)", () => {
    shapeIndex += 1;
    editedVertices.clear();
    dragVertex = -1;
    draw();
  });
  addButton("Zoom In (+)", () => {
    applyAroundPointer(scale(1.1));
    draw();
  });
  addButton("Zoom Out (-)", () => {
    applyAroundPointer(scale(1 / 1.1));
    draw();
  });
  addButton("Rotate Left (←)", () => {
    applyAroundPointer(rotate(-Math.PI / 20));
    draw();
  });
  addButton("Rotate Right (→)", () => {
    applyAroundPointer(rotate(Math.PI / 20));
    draw();
  });
  addButton("Reset View", () => {
    m[0] = 1;
    m[1] = 0;
    m[2] = 0;
    m[3] = 1;
    m[4] = 0;
    m[5] = 0;
    draw();
  });
  const hint = document.createElement("div");
  hint.className = "control-hint";
  hint.textContent = "Canvas: left-drag vertices. Arrows rotate around mouse, +/- zoom around mouse.";
  sidebar.appendChild(hint);
  function canvasPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: (e.clientY - rect.top) * sy
    };
  }
  function onKeyDown(e) {
    if (e.key === " ") {
      shapeIndex += 1;
      editedVertices.clear();
      dragVertex = -1;
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "+" || e.key === "=" || e.code === "NumpadAdd") {
      applyAroundPointer(scale(1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "-" || e.code === "NumpadSubtract") {
      applyAroundPointer(scale(1 / 1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "ArrowLeft") {
      applyAroundPointer(rotate(-Math.PI / 20));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === "ArrowRight") {
      applyAroundPointer(rotate(Math.PI / 20));
      draw();
      e.preventDefault();
    }
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const p = canvasPos2(e);
    pointerX = p.x;
    pointerY = p.y;
    const pickRadius = 10 / approxScale();
    let hit = flashPickVertex("flash_rasterizer2", W, H, buildParams(), p.x, p.y, pickRadius);
    dragUsesFlippedY = false;
    if (hit < 0) {
      const yFlip = H - p.y;
      hit = flashPickVertex("flash_rasterizer2", W, H, buildParams(), p.x, yFlip, pickRadius);
      if (hit >= 0) {
        dragUsesFlippedY = true;
      }
    }
    dragVertex = hit;
    canvas.setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onPointerMove(e) {
    const p = canvasPos2(e);
    pointerX = p.x;
    pointerY = p.y;
    if (dragVertex >= 0 && (e.buttons & 1) !== 0) {
      const dragY = dragUsesFlippedY ? H - p.y : p.y;
      const [lx, ly] = flashScreenToShape("flash_rasterizer2", W, H, buildParams(), p.x, dragY);
      editedVertices.set(dragVertex, { x: lx, y: ly });
      draw();
      e.preventDefault();
      return;
    }
    if (dragVertex >= 0 && (e.buttons & 1) === 0) {
      dragVertex = -1;
      draw();
    }
  }
  function onPointerUp() {
    dragUsesFlippedY = false;
    if (dragVertex >= 0) {
      dragVertex = -1;
      draw();
    }
  }
  window.addEventListener("keydown", onKeyDown);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  draw();
  return () => {
    window.removeEventListener("keydown", onKeyDown);
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
var init_flash_rasterizer2 = __esm(() => {
  init_render_canvas();
});

// src/demos/rasterizer_compound.ts
var exports_rasterizer_compound = {};
__export(exports_rasterizer_compound, {
  init: () => init59
});
function init59(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Rasterizer Compound", "Compound rasterizer with layer order control — matching C++ rasterizer_compound.cpp.");
  const W = 440;
  const H = 330;
  let strokeWidth = 10;
  let alpha1 = 1;
  let alpha2 = 1;
  let alpha3 = 1;
  let alpha4 = 1;
  let invertOrder = 0;
  let suppressSidebarEvents = false;
  let activeCanvasSlider = -1;
  function clamp(v, lo, hi) {
    return Math.max(lo, Math.min(hi, v));
  }
  function makeSlider(labelPrefix, min, max, step, decimals, get, set) {
    const group = document.createElement("div");
    group.className = "control-group";
    const label = document.createElement("label");
    label.className = "control-label";
    label.textContent = labelPrefix;
    group.appendChild(label);
    const input = document.createElement("input");
    input.className = "control-slider";
    input.type = "range";
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(get());
    group.appendChild(input);
    const valueEl = document.createElement("span");
    valueEl.className = "control-value";
    group.appendChild(valueEl);
    sidebar.appendChild(group);
    const state = { input, valueEl, min, max, decimals, get, set };
    const applyInput = () => {
      const v = clamp(parseFloat(input.value), min, max);
      set(v);
      draw();
    };
    input.addEventListener("input", () => {
      if (suppressSidebarEvents)
        return;
      applyInput();
    });
    return state;
  }
  const sliders = [
    makeSlider("Width", -20, 50, 0.01, 2, () => strokeWidth, (v) => {
      strokeWidth = v;
    }),
    makeSlider("Alpha1", 0, 1, 0.001, 3, () => alpha1, (v) => {
      alpha1 = v;
    }),
    makeSlider("Alpha2", 0, 1, 0.001, 3, () => alpha2, (v) => {
      alpha2 = v;
    }),
    makeSlider("Alpha3", 0, 1, 0.001, 3, () => alpha3, (v) => {
      alpha3 = v;
    }),
    makeSlider("Alpha4", 0, 1, 0.001, 3, () => alpha4, (v) => {
      alpha4 = v;
    })
  ];
  const cbDiv = document.createElement("div");
  cbDiv.className = "control-group";
  const cb = document.createElement("input");
  cb.type = "checkbox";
  cb.id = "rc_invert";
  cb.checked = false;
  cb.addEventListener("change", () => {
    if (suppressSidebarEvents)
      return;
    invertOrder = cb.checked ? 1 : 0;
    draw();
  });
  const cbLabel = document.createElement("label");
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = " Invert Z-Order";
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);
  function syncSidebarFromState() {
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
    renderToCanvas({
      demoName: "rasterizer_compound",
      canvas,
      width: W,
      height: H,
      params: [strokeWidth, alpha1, alpha2, alpha3, alpha4, invertOrder],
      timeDisplay: timeEl
    });
    syncSidebarFromState();
  }
  function canvasPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTopDown = (e.clientY - rect.top) * sy;
    return { x, yAgg: H - yTopDown };
  }
  const canvasSliderDefs = [
    { x1: 190, y1: 5, x2: 430, y2: 12, state: sliders[0] },
    { x1: 5, y1: 5, x2: 180, y2: 12, state: sliders[1] },
    { x1: 5, y1: 25, x2: 180, y2: 32, state: sliders[2] },
    { x1: 5, y1: 45, x2: 180, y2: 52, state: sliders[3] },
    { x1: 5, y1: 65, x2: 180, y2: 72, state: sliders[4] }
  ];
  function sliderIndexAt(x, yAgg) {
    for (let i = 0;i < canvasSliderDefs.length; i += 1) {
      const s = canvasSliderDefs[i];
      const yPad = (s.y2 - s.y1) * 0.8;
      if (x >= s.x1 && x <= s.x2 && yAgg >= s.y1 - yPad && yAgg <= s.y2 + yPad) {
        return i;
      }
    }
    return -1;
  }
  function setSliderFromCanvas(i, x) {
    const def = canvasSliderDefs[i];
    const t = clamp((x - def.x1) / (def.x2 - def.x1), 0, 1);
    const v = def.state.min + t * (def.state.max - def.state.min);
    def.state.set(v);
  }
  function checkboxHit(x, yAgg) {
    const x1 = 190;
    const y1 = 25;
    const x2 = 330;
    const y2 = 40;
    return x >= x1 && x <= x2 && yAgg >= y1 && yAgg <= y2;
  }
  function onPointerDown(e) {
    if (e.button !== 0)
      return;
    const p = canvasPos2(e);
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
  function onPointerMove(e) {
    if (activeCanvasSlider < 0 || (e.buttons & 1) === 0)
      return;
    const p = canvasPos2(e);
    setSliderFromCanvas(activeCanvasSlider, p.x);
    draw();
    e.preventDefault();
  }
  function onPointerUp() {
    activeCanvasSlider = -1;
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  draw();
  return () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
  };
}
var init_rasterizer_compound = __esm(() => {
  init_render_canvas();
});

// src/demos/gouraud_mesh.ts
var exports_gouraud_mesh = {};
__export(exports_gouraud_mesh, {
  init: () => init60
});
function init60(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Gouraud Mesh", "Gouraud-shaded triangle mesh with compound rasterizer — matching C++ gouraud_mesh.cpp.");
  const W = 512, H = 400;
  let cols = 8, rows = 8, seed = 0;
  function draw() {
    renderToCanvas({
      demoName: "gouraud_mesh",
      canvas,
      width: W,
      height: H,
      params: [cols, rows, seed],
      timeDisplay: timeEl
    });
  }
  addSlider(sidebar, "Columns", 3, 20, 8, 1, (v) => {
    cols = v;
    draw();
  });
  addSlider(sidebar, "Rows", 3, 20, 8, 1, (v) => {
    rows = v;
    draw();
  });
  addSlider(sidebar, "Color Seed", 0, 100, 0, 1, (v) => {
    seed = v;
    draw();
  });
  draw();
}
var init_gouraud_mesh = __esm(() => {
  init_render_canvas();
});

// src/demos/image_resample.ts
var exports_image_resample = {};
__export(exports_image_resample, {
  init: () => init61
});
function init61(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Image Resample", "Image resampling with affine and perspective transforms — matching C++ image_resample.cpp.");
  const W = 512, H = 400;
  let mode = 0, blur = 1;
  function draw() {
    renderToCanvas({
      demoName: "image_resample",
      canvas,
      width: W,
      height: H,
      params: [mode, blur],
      timeDisplay: timeEl
    });
  }
  addSlider(sidebar, "Mode (0-3)", 0, 3, 0, 1, (v) => {
    mode = v;
    draw();
  });
  addSlider(sidebar, "Blur", 0.5, 2, 1, 0.05, (v) => {
    blur = v;
    draw();
  });
  draw();
}
var init_image_resample = __esm(() => {
  init_render_canvas();
});

// src/demos/alpha_mask2.ts
var exports_alpha_mask2 = {};
__export(exports_alpha_mask2, {
  init: () => init62
});
function init62(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "Alpha Mask 2", "Alpha mask with random ellipses modulating lion rendering — matching C++ alpha_mask2.cpp.");
  const W = 512, H = 400;
  let numEllipses = 10;
  let angle = 0;
  let scale = 1;
  let skewX = 0;
  let skewY = 0;
  let dragging = false;
  function draw() {
    renderToCanvas({
      demoName: "alpha_mask2",
      canvas,
      width: W,
      height: H,
      params: [numEllipses, angle, scale, skewX, skewY],
      timeDisplay: timeEl
    });
  }
  function canvasPos2(e) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTop = (e.clientY - rect.top) * sy;
    return { x, y: H - yTop };
  }
  function transform(x, y) {
    const dx = x - W / 2;
    const dy = y - H / 2;
    angle = Math.atan2(dy, dx);
    scale = Math.hypot(dx, dy) / 100;
    if (scale < 0.01)
      scale = 0.01;
  }
  function applyPointer(flags, x, y) {
    if ((flags & 1) !== 0) {
      transform(x, y);
    }
    if ((flags & 2) !== 0) {
      skewX = x;
      skewY = y;
    }
  }
  const slNum = addSlider(sidebar, "N", 5, 100, 10, 1, (v) => {
    numEllipses = Math.round(v);
    draw();
  });
  const canvasControls = [
    {
      type: "slider",
      x1: 5,
      y1: 5,
      x2: 150,
      y2: 12,
      min: 5,
      max: 100,
      sidebarEl: slNum,
      onChange: (v) => {
        numEllipses = Math.round(v);
        draw();
      }
    }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  function onPointerDown(e) {
    const p = canvasPos2(e);
    canvas.setPointerCapture(e.pointerId);
    dragging = true;
    if (e.button === 2) {
      applyPointer(2, p.x, p.y);
      draw();
      e.preventDefault();
      return;
    }
    if (e.button === 0) {
      applyPointer(1, p.x, p.y);
      draw();
    }
  }
  function onPointerMove(e) {
    if (!dragging)
      return;
    const p = canvasPos2(e);
    let flags = 0;
    if ((e.buttons & 1) !== 0)
      flags |= 1;
    if ((e.buttons & 2) !== 0)
      flags |= 2;
    if (flags === 0)
      return;
    applyPointer(flags, p.x, p.y);
    draw();
  }
  function onPointerUp() {
    dragging = false;
  }
  function onContextMenu(e) {
    e.preventDefault();
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("contextmenu", onContextMenu);
  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    canvas.removeEventListener("contextmenu", onContextMenu);
  };
}
var init_alpha_mask2 = __esm(() => {
  init_render_canvas();
});

// src/demos/truetype_test.ts
var exports_truetype_test = {};
__export(exports_truetype_test, {
  init: () => init63
});
function init63(container) {
  const { canvas, sidebar, timeEl } = createDemoLayout(container, "TrueType LCD Subpixel", "LCD subpixel font rendering with faux weight/italic, gamma, and multiple typefaces. Port of C++ truetype_test_02_win.");
  const W = 640, H = 560;
  let typefaceIdx = 4;
  let fontScale = 1.43;
  let fauxItalic = 0;
  let fauxWeight = 0;
  let interval = 0;
  let widthVal = 1;
  let gamma = 1;
  let primaryWt = 1 / 3;
  let grayscale = false;
  let hinting = true;
  let kerning = true;
  let invert = false;
  function draw() {
    renderToCanvas({
      demoName: "truetype_test",
      canvas,
      width: W,
      height: H,
      params: [
        typefaceIdx,
        fontScale,
        fauxItalic,
        fauxWeight,
        interval,
        widthVal,
        gamma,
        primaryWt,
        grayscale ? 1 : 0,
        hinting ? 1 : 0,
        kerning ? 1 : 0,
        invert ? 1 : 0
      ],
      timeDisplay: timeEl,
      flipY: true
    });
  }
  const radioTypeface = addRadioGroup(sidebar, "Typeface", ["Arial", "Tahoma", "Verdana", "Times", "Georgia"], 4, (i) => {
    typefaceIdx = i;
    draw();
  });
  const slFontScale = addSlider(sidebar, "Font Scale", 0.5, 2, 1.43, 0.01, (v) => {
    fontScale = v;
    draw();
  });
  const slFauxItalic = addSlider(sidebar, "Faux Italic", -1, 1, 0, 0.01, (v) => {
    fauxItalic = v;
    draw();
  });
  const slFauxWeight = addSlider(sidebar, "Faux Weight", -1, 1, 0, 0.01, (v) => {
    fauxWeight = v;
    draw();
  });
  const slInterval = addSlider(sidebar, "Interval", -0.2, 0.2, 0, 0.001, (v) => {
    interval = v;
    draw();
  });
  const slWidth = addSlider(sidebar, "Width", 0.75, 1.25, 1, 0.01, (v) => {
    widthVal = v;
    draw();
  });
  const slGamma = addSlider(sidebar, "Gamma", 0.5, 2.5, 1, 0.01, (v) => {
    gamma = v;
    draw();
  });
  const slPrimaryWt = addSlider(sidebar, "Primary Weight", 0, 1, 1 / 3, 0.01, (v) => {
    primaryWt = v;
    draw();
  });
  const cbGrayscale = addCheckbox(sidebar, "Grayscale", false, (v) => {
    grayscale = v;
    draw();
  });
  const cbHinting = addCheckbox(sidebar, "Hinting", true, (v) => {
    hinting = v;
    draw();
  });
  const cbKerning = addCheckbox(sidebar, "Kerning", true, (v) => {
    kerning = v;
    draw();
  });
  const cbInvert = addCheckbox(sidebar, "Invert", false, (v) => {
    invert = v;
    draw();
  });
  const canvasControls = [
    { type: "radio", x1: 5, y1: 5, x2: 155, y2: 110, numItems: 5, sidebarEls: radioTypeface, onChange: (i) => {
      typefaceIdx = i;
      draw();
    } },
    { type: "slider", x1: 160, y1: 10, x2: 635, y2: 17, min: 0.5, max: 2, sidebarEl: slFontScale, onChange: (v) => {
      fontScale = v;
      draw();
    } },
    { type: "slider", x1: 160, y1: 25, x2: 635, y2: 32, min: -1, max: 1, sidebarEl: slFauxItalic, onChange: (v) => {
      fauxItalic = v;
      draw();
    } },
    { type: "slider", x1: 160, y1: 40, x2: 635, y2: 47, min: -1, max: 1, sidebarEl: slFauxWeight, onChange: (v) => {
      fauxWeight = v;
      draw();
    } },
    { type: "slider", x1: 260, y1: 55, x2: 635, y2: 62, min: -0.2, max: 0.2, sidebarEl: slInterval, onChange: (v) => {
      interval = v;
      draw();
    } },
    { type: "slider", x1: 260, y1: 70, x2: 635, y2: 77, min: 0.75, max: 1.25, sidebarEl: slWidth, onChange: (v) => {
      widthVal = v;
      draw();
    } },
    { type: "slider", x1: 260, y1: 85, x2: 635, y2: 92, min: 0.5, max: 2.5, sidebarEl: slGamma, onChange: (v) => {
      gamma = v;
      draw();
    } },
    { type: "slider", x1: 260, y1: 100, x2: 635, y2: 107, min: 0, max: 1, sidebarEl: slPrimaryWt, onChange: (v) => {
      primaryWt = v;
      draw();
    } },
    { type: "checkbox", x1: 160, y1: 50, x2: 250, y2: 64, sidebarEl: cbGrayscale, onChange: (v) => {
      grayscale = v;
      draw();
    } },
    { type: "checkbox", x1: 160, y1: 65, x2: 250, y2: 79, sidebarEl: cbHinting, onChange: (v) => {
      hinting = v;
      draw();
    } },
    { type: "checkbox", x1: 160, y1: 80, x2: 250, y2: 94, sidebarEl: cbKerning, onChange: (v) => {
      kerning = v;
      draw();
    } },
    { type: "checkbox", x1: 160, y1: 95, x2: 250, y2: 109, sidebarEl: cbInvert, onChange: (v) => {
      invert = v;
      draw();
    } }
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: "bottom-left" });
  draw();
  return () => cleanupCC();
}
var init_truetype_test = __esm(() => {
  init_render_canvas();
});
// src/legacy/sections.ts
var HISTORY_SECTIONS = [
  {
    route: "history/home",
    title: "Main Page",
    sourcePath: "index.html",
    description: "Original Anti-Grain Geometry landing page and primary navigation.",
    rustRoutes: ["home"]
  },
  {
    route: "history/about",
    title: "About",
    sourcePath: "about/index.html",
    description: "Project philosophy, original AGG context, and the current Rust-port direction.",
    rustRoutes: ["home", "lion", "gradients"],
    portUpdateNote: 'Rust-port update: references to the historical General Polygon Clipper are kept for context only. This project uses modern Rust approaches (including <a href="https://github.com/tirithen/clipper2" target="_blank" rel="noreferrer">clipper2-rust</a> workflows) and does not ship GPC.'
  },
  {
    route: "history/news",
    title: "News",
    sourcePath: "news/index.html",
    description: "Historical release timeline and development updates.",
    portUpdateNote: "Rust-port update: legacy news entries may mention GPC as part of original AGG history; this port does not include GPC and follows Rust-native alternatives."
  },
  {
    route: "history/license",
    title: "License",
    sourcePath: "license/index.html",
    description: "Original AGG licensing notes and historical license text.",
    archiveNote: "This section is preserved for historical context. For this Rust port, see repository licensing details.",
    portUpdateNote: 'Rust-port update: the GPC component/license text is preserved historically, but GPC is not used in this Rust port. The Rust port uses <a href="https://github.com/tirithen/clipper2" target="_blank" rel="noreferrer">clipper2-rust</a>-based workflows for modern polygon operations where needed.'
  },
  {
    route: "history/download",
    title: "Download",
    sourcePath: "download/index.html",
    description: "Historical package archives and legacy distribution notes.",
    archiveNote: "Download links are historical snapshots. Prefer the Rust repository and releases for current work.",
    currentLinks: [
      { label: "Rust Port on GitHub", href: "https://github.com/larsbrubaker/agg-rust" },
      { label: "Rust Port Releases", href: "https://github.com/larsbrubaker/agg-rust/releases" }
    ]
  },
  {
    route: "history/screenshots",
    title: "Screenshots",
    sourcePath: "screenshots/index.html",
    description: "Image gallery from the original AGG website.",
    rustRoutes: ["home"]
  },
  {
    route: "history/demo",
    title: "Demo",
    sourcePath: "demo/index.html",
    description: "Original demo index and platform-era screenshots.",
    rustRoutes: ["home", "aa_demo", "lion", "compositing", "image_resample"],
    portUpdateNote: "Rust-port update: demo references to GPC-based workflows are historical; this Rust port uses non-GPC paths aligned with current Rust tooling."
  },
  {
    route: "history/svg",
    title: "SVG Viewer",
    sourcePath: "svg/index.html",
    description: "Legacy SVG viewer references and related materials."
  },
  {
    route: "history/docs",
    title: "Documentation",
    sourcePath: "doc/index.html",
    description: "Core AGG documentation overview and doc entry points.",
    portUpdateNote: "Rust-port update: documentation links referencing `conv_gpc` are historical AGG references and are not active design choices for this Rust port."
  },
  {
    route: "history/tips",
    title: "Tips and Tricks",
    sourcePath: "tips/index.html",
    description: "Practical techniques and usage notes from the original site."
  },
  {
    route: "history/research",
    title: "Research and Articles",
    sourcePath: "research/index.html",
    description: "Technical research articles and deep dives by Maxim Shemanarev."
  },
  {
    route: "history/svn",
    title: "SVN",
    sourcePath: "svn/index.html",
    description: "Legacy source control references from the original site."
  },
  {
    route: "history/sponsors",
    title: "Sponsors",
    sourcePath: "sponsors/index.html",
    description: "Organizations that supported AGG development."
  },
  {
    route: "history/customers",
    title: "Users and Customers",
    sourcePath: "customers/index.html",
    description: "Historical user and customer showcase."
  },
  {
    route: "history/links",
    title: "Links and Friends",
    sourcePath: "links/index.html",
    description: "Related sites and references from the AGG ecosystem.",
    portUpdateNote: "Rust-port update: third-party GPC links are kept as historical references only; current polygon workflows use Rust-native alternatives."
  },
  {
    route: "history/contact",
    title: "Contact",
    sourcePath: "mcseem/index.html",
    description: "Original author contact page and profile."
  }
];
var HISTORY_SECTION_BY_ROUTE = Object.fromEntries(HISTORY_SECTIONS.map((section) => [section.route, section]));
var HISTORY_SECTION_BY_SOURCE_PATH = Object.fromEntries(HISTORY_SECTIONS.map((section) => [section.sourcePath, section]));
function historySourcePathForRoute(route) {
  const section = HISTORY_SECTION_BY_ROUTE[route];
  if (section) {
    return section.sourcePath;
  }
  if (route.startsWith("history/page/")) {
    return route.slice("history/page/".length);
  }
  return null;
}
function isLocallyHostedHistorySourcePath(sourcePath) {
  return sourcePath.startsWith("research/") || sourcePath.startsWith("tips/");
}

// src/legacy/renderer.ts
var contentIndexPromise = null;
function getSectionRoute(route) {
  if (route === "history") {
    return "history/home";
  }
  if (route === "legacy")
    return "history/home";
  if (route.startsWith("legacy/"))
    return `history/${route.slice("legacy/".length)}`;
  return route;
}
function sourceUrlForPath(path) {
  return `https://agg.sourceforge.net/antigrain.com/${path}`;
}
async function loadContentIndex() {
  if (!contentIndexPromise) {
    contentIndexPromise = fetch("./public/history/content-index.json").then(async (response) => {
      if (!response.ok) {
        return null;
      }
      return await response.json();
    }).catch(() => null);
  }
  return contentIndexPromise;
}
function indexByRoute(index) {
  if (!index) {
    return {};
  }
  return Object.fromEntries(index.entries.map((entry) => [entry.route, entry]));
}
function rustLinksHtml(section) {
  if (!section.rustRoutes || section.rustRoutes.length === 0) {
    return "";
  }
  const links = section.rustRoutes.map((route) => `<a href="#/${route}" class="legacy-chip">${route}</a>`).join("");
  return `
    <div class="legacy-rust-links">
      <h4>Related Rust Routes</h4>
      <div class="legacy-chip-row">${links}</div>
    </div>
  `;
}
function tributeBanner(section, sourceUrl, sourcePath) {
  const archiveNote = section.archiveNote ? `<p class="legacy-note">${section.archiveNote}</p>` : "";
  const portUpdateNote = section.portUpdateNote ? `<p class="legacy-port-update">${section.portUpdateNote}</p>` : "";
  const currentLinks = section.currentLinks && section.currentLinks.length > 0 ? section.currentLinks.map((link) => `<a href="${link.href}" target="_blank" rel="noreferrer">${link.label}</a>`).join("") : "";
  const showOriginalLink = !isLocallyHostedHistorySourcePath(sourcePath);
  const originalLink = showOriginalLink ? `<a href="${sourceUrl}" target="_blank" rel="noreferrer">Original page on SourceForge mirror</a>` : `<span class="legacy-local-note">Original AGG content is hosted locally on this site.</span>`;
  return `
    <section class="legacy-banner">
      <h1>${section.title}</h1>
      <p>${section.description}</p>
      ${archiveNote}
      ${portUpdateNote}
      <div class="legacy-banner-links">
        ${originalLink}
        ${currentLinks}
      </div>
    </section>
  `;
}
function formatDate(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric"
  });
}
function livingUpdateHtml(sectionRoute, generatedAt) {
  if (sectionRoute !== "history/about") {
    return "";
  }
  const dateLabel = formatDate(generatedAt ?? new Date().toISOString());
  return `
    <section class="legacy-living-update">
      <h2>Rust Port: Current Work</h2>
      <p class="legacy-current-date">Status date: <strong>${dateLabel}</strong></p>
      <p>
        The original AGG project remains the foundation, and this site now tracks the
        active Rust port as a living continuation of that work.
      </p>
      <h3>Current focus</h3>
      <ul>
        <li>Porting and validating AGG modules with behavior parity against the original C++ implementation.</li>
        <li>Maintaining and expanding interactive WebAssembly demos while preserving AGG visual fidelity.</li>
        <li>Keeping this History and Living Project section in sync with current Rust-port progress.</li>
        <li>Using Rust-native polygon workflows (including <a href="https://github.com/tirithen/clipper2" target="_blank" rel="noreferrer">clipper2-rust</a> patterns); GPC is not part of this port.</li>
      </ul>
      <p>
        See the main demo index and project repository for the latest implementation status.
      </p>
    </section>
  `;
}
function contactPageHtml(sectionRoute) {
  if (sectionRoute !== "history/contact") {
    return "";
  }
  return `
    <section class="legacy-living-update legacy-contact-note">
      <h2>In Memory of Maxim Shemanarev</h2>
      <div class="legacy-contact-image-wrap">
        <img
          class="legacy-contact-image"
          src="./public/history/assets/mcseem/mcseem.jpg"
          alt="Maxim Shemanarev"
          loading="lazy"
        />
      </div>
      <p>
        Maxim Shemanarev, the original author of Anti-Grain Geometry (AGG), created one of the
        most influential software rendering libraries in modern graphics programming.
      </p>
      <p>
        This project is built with deep respect for his work and its long-lasting impact.
      </p>
      <p>
        Historical contact information from the original site is no longer valid.
      </p>
      <p>
        For questions about this Rust port, please contact
        <a href="mailto:larsbrubaker@gmail.com">larsbrubaker@gmail.com</a>.
      </p>
      <p>
        Learn more about AGG:
        <a href="https://en.wikipedia.org/wiki/Anti-Grain_Geometry" target="_blank" rel="noreferrer">Anti-Grain Geometry on Wikipedia</a>.
      </p>
    </section>
  `;
}
function mapDemoStemToRoute(stem, availableRoutes) {
  if (availableRoutes.has(stem)) {
    return stem;
  }
  if (stem === "gpc_test" && availableRoutes.has("scanline_boolean2")) {
    return "scanline_boolean2";
  }
  if (stem === "freetype_test" && availableRoutes.has("truetype_test")) {
    return "truetype_test";
  }
  return null;
}
function rustDemoSourceUrl(route) {
  return `https://github.com/larsbrubaker/agg-rust/blob/master/demo/src/demos/${route}.ts`;
}
function enhanceDemoTable(container) {
  const content = container.querySelector(".legacy-content");
  if (!content) {
    return;
  }
  const table = content.querySelector("table.tbl");
  if (!table) {
    return;
  }
  const oldIntroTables = Array.from(content.querySelectorAll("table")).filter((t) => t !== table);
  for (const t of oldIntroTables) {
    const hasDemoText = /demo examples|download|to be continued/i.test(t.textContent || "");
    if (hasDemoText) {
      t.remove();
    }
  }
  content.insertAdjacentHTML("afterbegin", `
      <section class="legacy-living-update history-demo-rust-intro">
        <h2>Rust Demo Index</h2>
        <p>
          This section tracks the Rust/WebAssembly demos in this repository. Use
          <strong>Open Rust demo</strong> to launch the live demo on this site and
          <strong>View Rust source</strong> to open the TypeScript demo entrypoint in GitHub.
        </p>
        <p>
          The original C++ demo references are preserved as historical context, but this page is
          now focused on the active Rust port.
        </p>
      </section>
    `);
  const availableRoutes = new Set(Array.from(document.querySelectorAll(".nav-link[data-route]")).map((el) => el.dataset.route || "").filter(Boolean));
  const headerRow = table.querySelector("tr");
  if (headerRow) {
    const ths = headerRow.querySelectorAll("th");
    if (ths.length >= 3) {
      ths[2].remove();
    }
  }
  const rows = Array.from(table.querySelectorAll("tr"));
  for (const row of rows) {
    const tds = row.querySelectorAll("td");
    if (tds.length >= 3) {
      tds[2].remove();
    }
    if (tds.length < 2) {
      continue;
    }
    const screenshotCell = tds[0];
    const descriptionCell = tds[1];
    const rowText = (descriptionCell.textContent || "").trim().toLowerCase();
    if (rowText.includes("all examples in one package")) {
      row.remove();
      continue;
    }
    const codeLink = descriptionCell.querySelector("code a");
    if (!codeLink) {
      continue;
    }
    const stemMatch = (codeLink.textContent || "").match(/([a-z0-9_]+)\.cpp/i);
    if (!stemMatch) {
      continue;
    }
    const stem = stemMatch[1];
    const route = mapDemoStemToRoute(stem, availableRoutes);
    const sourceLabel = descriptionCell.querySelector("code");
    const existingStatus = screenshotCell.querySelector(".history-demo-status");
    if (existingStatus) {
      existingStatus.remove();
    }
    const status = document.createElement("div");
    status.className = "history-demo-status";
    const existingLink = descriptionCell.querySelector(".history-demo-rust-link");
    if (existingLink) {
      existingLink.remove();
    }
    const rustLink = document.createElement("div");
    rustLink.className = "history-demo-rust-link";
    if (route) {
      const sourceUrl = rustDemoSourceUrl(route);
      codeLink.setAttribute("href", sourceUrl);
      codeLink.setAttribute("target", "_blank");
      codeLink.setAttribute("rel", "noreferrer");
      codeLink.textContent = `${route}.ts`;
      if (sourceLabel) {
        sourceLabel.insertAdjacentHTML("beforeend", ` <span class="history-demo-source-tag">Rust</span>`);
      }
      status.innerHTML = `<a href="#/${route}">Open Rust Demo</a>`;
      rustLink.innerHTML = `<a href="#/${route}">Open Rust demo</a> &middot; <a href="${sourceUrl}" target="_blank" rel="noreferrer">View Rust source</a>`;
    } else {
      codeLink.setAttribute("href", sourceUrlForPath(`demo/${stem}.cpp.html`));
      codeLink.setAttribute("target", "_blank");
      codeLink.setAttribute("rel", "noreferrer");
      status.classList.add("history-demo-soon");
      status.textContent = "Coming soon";
      rustLink.innerHTML = "Rust demo coming soon";
      rustLink.classList.add("history-demo-soon");
    }
    screenshotCell.appendChild(status);
    descriptionCell.appendChild(rustLink);
  }
}
function renderLanding(container) {
  const cards = HISTORY_SECTIONS.map((section) => {
    return `
      <a href="#/${section.route}" class="legacy-card">
        <h3>${section.title}</h3>
        <p>${section.description}</p>
      </a>
    `;
  }).join("");
  container.innerHTML = `
    <div class="legacy-page home-page">
      <section class="legacy-tribute-intro">
        <h1>AGG History and Living Project</h1>
        <p>
          This section preserves and modernizes the original Anti-Grain Geometry website by
          <strong>Maxim Shemanarev</strong> while documenting ongoing Rust-port progress.
        </p>
        <p>
          Think of it as a living history: original context, modern presentation, and direct
          links to active Rust demos and current implementation work.
        </p>
      </section>
      <section class="legacy-grid">${cards}</section>
    </div>
  `;
}
async function renderHistoryRoute(container, route) {
  if (route === "history" || route === "legacy") {
    renderLanding(container);
    return;
  }
  const sectionRoute = getSectionRoute(route);
  container.innerHTML = `
    <div class="legacy-page home-page">
      <p class="legacy-loading">Loading history content...</p>
    </div>
  `;
  const contentIndex = await loadContentIndex();
  const entriesByRoute = indexByRoute(contentIndex);
  const entry = entriesByRoute[sectionRoute];
  const sourcePath = entry?.sourcePath ?? historySourcePathForRoute(sectionRoute);
  if (!sourcePath) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        <h2>History page not found</h2>
        <p>Unknown route: ${route}</p>
      </div>
    `;
    return;
  }
  const section = HISTORY_SECTION_BY_ROUTE[sectionRoute] ?? {
    route: sectionRoute,
    title: entry?.title ?? "AGG Article",
    sourcePath,
    description: isLocallyHostedHistorySourcePath(sourcePath) ? "Historical AGG article hosted locally in the project history section." : "Historical AGG page."
  };
  const sourceUrl = entry?.sourceUrl ?? sourceUrlForPath(sourcePath);
  if (!entry) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        ${tributeBanner(section, sourceUrl, sourcePath)}
        ${livingUpdateHtml(sectionRoute, contentIndex?.generatedAt)}
        ${contactPageHtml(sectionRoute)}
        <div class="legacy-fallback">
          <p>
            Local generated content is not available yet for this page.
            Run the demo build to generate tribute fragments.
          </p>
          <p>
            <a href="${sourceUrl}" target="_blank" rel="noreferrer">Open original page</a>
          </p>
        </div>
        ${rustLinksHtml(section)}
      </div>
    `;
    return;
  }
  const response = await fetch(entry.contentPath);
  if (!response.ok) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        ${tributeBanner(section, sourceUrl, sourcePath)}
        ${livingUpdateHtml(sectionRoute, contentIndex?.generatedAt)}
        ${contactPageHtml(sectionRoute)}
        <div class="legacy-fallback">
          <p>Generated content could not be loaded from <code>${entry.contentPath}</code>.</p>
          <p><a href="${sourceUrl}" target="_blank" rel="noreferrer">Open original page</a></p>
        </div>
        ${rustLinksHtml(section)}
      </div>
    `;
    return;
  }
  const fragment = await response.text();
  const content = sectionRoute === "history/contact" ? "" : `<article class="legacy-content">${fragment}</article>`;
  container.innerHTML = `
    <div class="legacy-page home-page">
      ${tributeBanner(section, sourceUrl, sourcePath)}
      ${livingUpdateHtml(sectionRoute, entry.generatedAt)}
      ${contactPageHtml(sectionRoute)}
      ${content}
      ${rustLinksHtml(section)}
    </div>
  `;
  if (sectionRoute === "history/demo") {
    enhanceDemoTable(container);
  }
}

// src/main.ts
var demoModules = {
  lion: () => Promise.resolve().then(() => (init_lion(), exports_lion)),
  gradients: () => Promise.resolve().then(() => (init_gradients(), exports_gradients)),
  gouraud: () => Promise.resolve().then(() => (init_gouraud(), exports_gouraud)),
  conv_stroke: () => Promise.resolve().then(() => (init_conv_stroke(), exports_conv_stroke)),
  bezier_div: () => Promise.resolve().then(() => (init_bezier_div(), exports_bezier_div)),
  circles: () => Promise.resolve().then(() => (init_circles(), exports_circles)),
  rounded_rect: () => Promise.resolve().then(() => (init_rounded_rect(), exports_rounded_rect)),
  aa_demo: () => Promise.resolve().then(() => (init_aa_demo(), exports_aa_demo)),
  gamma_correction: () => Promise.resolve().then(() => (init_gamma_correction(), exports_gamma_correction)),
  line_thickness: () => Promise.resolve().then(() => (init_line_thickness(), exports_line_thickness)),
  rasterizers: () => Promise.resolve().then(() => (init_rasterizers(), exports_rasterizers)),
  conv_contour: () => Promise.resolve().then(() => (init_conv_contour(), exports_conv_contour)),
  conv_dash: () => Promise.resolve().then(() => (init_conv_dash(), exports_conv_dash)),
  perspective: () => Promise.resolve().then(() => (init_perspective(), exports_perspective)),
  image_fltr_graph: () => Promise.resolve().then(() => (init_image_fltr_graph(), exports_image_fltr_graph)),
  image1: () => Promise.resolve().then(() => (init_image1(), exports_image1)),
  image_filters: () => Promise.resolve().then(() => (init_image_filters(), exports_image_filters)),
  gradient_focal: () => Promise.resolve().then(() => (init_gradient_focal(), exports_gradient_focal)),
  idea: () => Promise.resolve().then(() => (init_idea(), exports_idea)),
  graph_test: () => Promise.resolve().then(() => (init_graph_test(), exports_graph_test)),
  gamma_tuner: () => Promise.resolve().then(() => (init_gamma_tuner(), exports_gamma_tuner)),
  image_filters2: () => Promise.resolve().then(() => (init_image_filters2(), exports_image_filters2)),
  conv_dash_marker: () => Promise.resolve().then(() => (init_conv_dash_marker(), exports_conv_dash_marker)),
  aa_test: () => Promise.resolve().then(() => (init_aa_test(), exports_aa_test)),
  bspline: () => Promise.resolve().then(() => (init_bspline(), exports_bspline)),
  image_perspective: () => Promise.resolve().then(() => (init_image_perspective(), exports_image_perspective)),
  alpha_mask: () => Promise.resolve().then(() => (init_alpha_mask(), exports_alpha_mask)),
  alpha_gradient: () => Promise.resolve().then(() => (init_alpha_gradient(), exports_alpha_gradient)),
  image_alpha: () => Promise.resolve().then(() => (init_image_alpha(), exports_image_alpha)),
  alpha_mask3: () => Promise.resolve().then(() => (init_alpha_mask3(), exports_alpha_mask3)),
  image_transforms: () => Promise.resolve().then(() => (init_image_transforms(), exports_image_transforms)),
  mol_view: () => Promise.resolve().then(() => (init_mol_view(), exports_mol_view)),
  raster_text: () => Promise.resolve().then(() => (init_raster_text(), exports_raster_text)),
  gamma_ctrl: () => Promise.resolve().then(() => (init_gamma_ctrl(), exports_gamma_ctrl)),
  trans_polar: () => Promise.resolve().then(() => (init_trans_polar(), exports_trans_polar)),
  multi_clip: () => Promise.resolve().then(() => (init_multi_clip(), exports_multi_clip)),
  simple_blur: () => Promise.resolve().then(() => (init_simple_blur(), exports_simple_blur)),
  blur: () => Promise.resolve().then(() => (init_blur(), exports_blur)),
  trans_curve1: () => Promise.resolve().then(() => (init_trans_curve1(), exports_trans_curve1)),
  trans_curve2: () => Promise.resolve().then(() => (init_trans_curve2(), exports_trans_curve2)),
  lion_lens: () => Promise.resolve().then(() => (init_lion_lens(), exports_lion_lens)),
  distortions: () => Promise.resolve().then(() => (init_distortions(), exports_distortions)),
  blend_color: () => Promise.resolve().then(() => (init_blend_color(), exports_blend_color)),
  component_rendering: () => Promise.resolve().then(() => (init_component_rendering(), exports_component_rendering)),
  polymorphic_renderer: () => Promise.resolve().then(() => (init_polymorphic_renderer(), exports_polymorphic_renderer)),
  scanline_boolean: () => Promise.resolve().then(() => (init_scanline_boolean(), exports_scanline_boolean)),
  scanline_boolean2: () => Promise.resolve().then(() => (init_scanline_boolean2(), exports_scanline_boolean2)),
  gpc_test: () => Promise.resolve().then(() => (init_scanline_boolean2(), exports_scanline_boolean2)),
  pattern_fill: () => Promise.resolve().then(() => (init_pattern_fill(), exports_pattern_fill)),
  pattern_perspective: () => Promise.resolve().then(() => (init_pattern_perspective(), exports_pattern_perspective)),
  pattern_resample: () => Promise.resolve().then(() => (init_pattern_resample(), exports_pattern_resample)),
  lion_outline: () => Promise.resolve().then(() => (init_lion_outline(), exports_lion_outline)),
  rasterizers2: () => Promise.resolve().then(() => (init_rasterizers2(), exports_rasterizers2)),
  line_patterns: () => Promise.resolve().then(() => (init_line_patterns(), exports_line_patterns)),
  line_patterns_clip: () => Promise.resolve().then(() => (init_line_patterns_clip(), exports_line_patterns_clip)),
  compositing: () => Promise.resolve().then(() => (init_compositing(), exports_compositing)),
  compositing2: () => Promise.resolve().then(() => (init_compositing2(), exports_compositing2)),
  flash_rasterizer: () => Promise.resolve().then(() => (init_flash_rasterizer(), exports_flash_rasterizer)),
  flash_rasterizer2: () => Promise.resolve().then(() => (init_flash_rasterizer2(), exports_flash_rasterizer2)),
  rasterizer_compound: () => Promise.resolve().then(() => (init_rasterizer_compound(), exports_rasterizer_compound)),
  gouraud_mesh: () => Promise.resolve().then(() => (init_gouraud_mesh(), exports_gouraud_mesh)),
  image_resample: () => Promise.resolve().then(() => (init_image_resample(), exports_image_resample)),
  alpha_mask2: () => Promise.resolve().then(() => (init_alpha_mask2(), exports_alpha_mask2)),
  truetype_test: () => Promise.resolve().then(() => (init_truetype_test(), exports_truetype_test))
};
var thumbnails = {
  aa_demo: "aa_demo.gif",
  aa_test: "aa_test.png",
  alpha_gradient: "alpha_gradient.png",
  alpha_mask: "alpha_mask.gif",
  alpha_mask2: "alpha_mask2.jpg",
  alpha_mask3: "alpha_mask3.gif",
  bezier_div: "bezier_div.png",
  blend_color: "compositing.png",
  blur: "blur.png",
  bspline: "bezier_div.png",
  circles: "circles.gif",
  component_rendering: "component_rendering.gif",
  compositing: "compositing.png",
  compositing2: "compositing2.png",
  conv_contour: "conv_contour.gif",
  conv_dash: "conv_dash_marker.gif",
  conv_dash_marker: "conv_dash_marker.gif",
  conv_stroke: "conv_stroke.gif",
  distortions: "distortions.png",
  flash_rasterizer: "flash_rasterizer.png",
  flash_rasterizer2: "flash_rasterizer2.png",
  gamma_correction: "gamma_correction.gif",
  gamma_ctrl: "gamma_ctrl.gif",
  gamma_tuner: "gamma_tuner.png",
  gouraud: "gouraud.png",
  gouraud_mesh: "gouraud_mesh.png",
  gradient_focal: "gradient_focal.png",
  gradients: "gradients.png",
  graph_test: "graph_test.gif",
  idea: "idea.gif",
  image_alpha: "image_alpha.png",
  image_filters: "image_filters.jpg",
  image_filters2: "image_filters2.png",
  image_fltr_graph: "image_fltr_graph.gif",
  image_perspective: "image_perspective.jpg",
  image_resample: "image_resample.jpg",
  image_transforms: "image_transforms.jpg",
  image1: "image1.jpg",
  line_patterns: "line_patterns.gif",
  line_patterns_clip: "line_patterns_clip.png",
  line_thickness: "conv_stroke.gif",
  lion: "lion.png",
  lion_lens: "lion_lens.gif",
  lion_outline: "lion_outline.gif",
  mol_view: "mol_view.gif",
  multi_clip: "multi_clip.png",
  pattern_fill: "pattern_fill.gif",
  pattern_perspective: "pattern_perspective.jpg",
  pattern_resample: "pattern_resample.jpg",
  perspective: "perspective.gif",
  polymorphic_renderer: "polymorphic_renderer.gif",
  raster_text: "raster_text.gif",
  rasterizer_compound: "rasterizer_compound.png",
  rasterizers: "rasterizers.gif",
  rasterizers2: "rasterizers2.gif",
  rounded_rect: "rounded_rect.gif",
  scanline_boolean: "scanline_boolean.gif",
  scanline_boolean2: "scanline_boolean2.gif",
  gpc_test: "scanline_boolean2.gif",
  simple_blur: "simple_blur.gif",
  trans_curve1: "trans_curve1.gif",
  trans_curve2: "trans_curve2.gif",
  truetype_test: "truetype_test.png",
  trans_polar: "trans_polar.gif"
};
function thumbImg(route, cssClass) {
  const file = thumbnails[route];
  if (file) {
    return `<img class="${cssClass}" src="./public/thumbnails/${file}" alt="${route}" loading="lazy">`;
  }
  return `<span class="${cssClass === "card-thumb" ? "card-thumb-fallback" : "nav-icon"}">&#9670;</span>`;
}
var currentCleanup = null;
var currentRouteKey = null;
var routeScrollPositions = new Map;
function canonicalRoute(route) {
  if (route === "legacy")
    return "history";
  if (route.startsWith("legacy/"))
    return `history/${route.slice("legacy/".length)}`;
  return route;
}
function saveCurrentRouteScroll() {
  if (!currentRouteKey)
    return;
  routeScrollPositions.set(currentRouteKey, window.scrollY || window.pageYOffset || 0);
}
function restoreRouteScroll(routeKey) {
  const targetY = routeScrollPositions.get(routeKey) ?? 0;
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      window.scrollTo({ top: targetY, left: 0, behavior: "auto" });
    });
  });
}
function finalizeNavigation(routeKey) {
  currentRouteKey = routeKey;
  restoreRouteScroll(routeKey);
}
function getRoute() {
  const hash = window.location.hash.slice(2) || "";
  return hash || "home";
}
function updateNav(route) {
  document.querySelectorAll(".nav-link").forEach((el) => {
    const r = el.dataset.route;
    const isActive = r === route;
    el.classList.toggle("active", isActive);
    if (isActive) {
      const group = el.closest(".nav-group");
      if (group) {
        group.classList.add("open");
        const btn = group.querySelector(".nav-section");
        if (btn)
          btn.setAttribute("aria-expanded", "true");
        const KEY = "agg-sidebar-sections";
        try {
          const saved = JSON.parse(localStorage.getItem(KEY) || "{}");
          saved[group.dataset.section] = true;
          localStorage.setItem(KEY, JSON.stringify(saved));
        } catch (e) {}
      }
    }
  });
}
var demoCards = [
  { route: "history", title: "AGG History and Living Project", desc: "The original AGG site in a modernized light theme, connected directly to ongoing Rust-port work and interactive demos." },
  { route: "lion", title: "Lion", desc: "The classic AGG lion &mdash; a complex vector graphic with rotation and scaling controls." },
  { route: "gradients", title: "Gradients", desc: "Linear and radial gradient fills with multi-stop color interpolation." },
  { route: "gouraud", title: "Gouraud Shading", desc: "Smooth color interpolation across triangles using Gouraud shading." },
  { route: "conv_stroke", title: "Conv Stroke", desc: "Line joins (miter, round, bevel), caps, and dashed overlay with draggable vertices." },
  { route: "bezier_div", title: "Bezier Div", desc: "Cubic B&eacute;zier curve subdivision with draggable control points and width control." },
  { route: "circles", title: "Circles", desc: "Random anti-aliased circles with configurable count, size range, and seed." },
  { route: "rounded_rect", title: "Rounded Rect", desc: "Draggable rounded rectangle with adjustable corner radius." },
  { route: "aa_demo", title: "AA Demo", desc: "Anti-aliasing visualization &mdash; enlarged pixel view of a triangle." },
  { route: "gamma_correction", title: "Gamma Correction", desc: "Gamma curve visualization with concentric colored ellipses." },
  { route: "line_thickness", title: "Line Thickness", desc: "Lines at varying sub-pixel widths from 0.1 to 5.0 pixels." },
  { route: "rasterizers", title: "Rasterizers", desc: "Filled and stroked triangle with alpha control." },
  { route: "conv_contour", title: "Conv Contour", desc: 'Letter "A" with adjustable contour width and orientation control.' },
  { route: "conv_dash", title: "Conv Dash", desc: "Dashed stroke patterns with cap styles on a draggable triangle." },
  { route: "perspective", title: "Perspective", desc: "Lion with bilinear/perspective quad transform &mdash; drag corners to warp." },
  { route: "image_fltr_graph", title: "Filter Graph", desc: "Image filter kernel weight function visualization &mdash; 16 filters." },
  { route: "image1", title: "Image Transforms", desc: "Procedural sphere image with affine rotation/scaling through a bilinear filter." },
  { route: "image_filters", title: "Image Filters", desc: "Iterative rotation showing filter quality degradation &mdash; 17 filter types." },
  { route: "gradient_focal", title: "Gradient Focal", desc: "Radial gradient with moveable focal point and reflect adaptor." },
  { route: "idea", title: "Idea", desc: "Rotating light bulb icon with even-odd fill, draft, and roundoff options." },
  { route: "graph_test", title: "Graph Test", desc: "Random graph with 200 nodes and 100 edges &mdash; 5 rendering modes." },
  { route: "gamma_tuner", title: "Gamma Tuner", desc: "Gradient background with alpha pattern and gamma correction controls." },
  { route: "image_filters2", title: "Image Filters 2", desc: "4x4 test image filtered through 17 filter types with graph visualization." },
  { route: "conv_dash_marker", title: "Dash Marker", desc: "Dashed strokes with cap styles on a draggable triangle." },
  { route: "aa_test", title: "AA Test", desc: "Anti-aliasing quality test &mdash; radial lines, gradient lines, Gouraud triangles." },
  { route: "bspline", title: "B-Spline", desc: "B-spline curve through 6 draggable control points with adjustable density." },
  { route: "image_perspective", title: "Image Perspective", desc: "Image transformed through affine/bilinear/perspective quad corners." },
  { route: "alpha_mask", title: "Alpha Mask", desc: "Lion with elliptical alpha mask &mdash; rotate, scale, and skew." },
  { route: "alpha_gradient", title: "Alpha Gradient", desc: "Gradient with alpha curve control over random ellipse background." },
  { route: "image_alpha", title: "Image Alpha", desc: "Image with brightness-to-alpha mapping over random ellipses." },
  { route: "alpha_mask3", title: "Alpha Mask 3", desc: "Alpha mask polygon clipping with AND/SUB operations." },
  { route: "image_transforms", title: "Image Transforms", desc: "Star polygon textured with image through 7 transform modes." },
  { route: "mol_view", title: "Molecule Viewer", desc: "Molecular structure viewer with rotate, scale, and pan controls." },
  { route: "raster_text", title: "Raster Text", desc: "All 34 embedded bitmap fonts rendered with sample text strings." },
  { route: "gamma_ctrl", title: "Gamma Control", desc: "Interactive gamma spline widget with stroked ellipses." },
  { route: "trans_polar", title: "Polar Transform", desc: "Slider control warped through polar coordinates with spiral effect." },
  { route: "multi_clip", title: "Multi Clip", desc: "Lion rendered through N&times;N clip regions with random shapes." },
  { route: "simple_blur", title: "Simple Blur", desc: "Lion with 3&times;3 box blur &mdash; original vs blurred comparison." },
  { route: "blur", title: "Blur", desc: "Stack blur and recursive blur on colored shapes with adjustable radius." },
  { route: "trans_curve1", title: "Text on Curve", desc: "Text warped along a B-spline curve with draggable control points." },
  { route: "trans_curve2", title: "Text on Curve 2", desc: "Text warped along a curve with adjustable approximation scale." },
  { route: "lion_lens", title: "Lion Lens", desc: "Magnifying lens distortion on the lion using trans_warp_magnifier." },
  { route: "distortions", title: "Distortions", desc: "Animated wave and swirl distortions on image and gradient sources with adjustable controls." },
  { route: "blend_color", title: "Blend Color", desc: "Color blending modes with alpha compositing demonstration." },
  { route: "component_rendering", title: "Component Rendering", desc: "Per-component rendering of individual color channels." },
  { route: "polymorphic_renderer", title: "Polymorphic Renderer", desc: "Multiple renderer types dispatched through a common interface." },
  { route: "scanline_boolean", title: "Scanline Boolean", desc: "Boolean operations (AND, OR, XOR, SUB) on scanline shapes." },
  { route: "scanline_boolean2", title: "Scanline Boolean 2", desc: "Advanced boolean polygon operations with multiple shapes." },
  { route: "gpc_test", title: "GPC Test (Rust Replacement)", desc: "Original GPC test workflow mapped to the Rust boolean-operations demo implementation." },
  { route: "pattern_fill", title: "Pattern Fill", desc: "Tiled pattern fill on polygon shapes." },
  { route: "pattern_perspective", title: "Pattern Perspective", desc: "Pattern fill with perspective transformation." },
  { route: "pattern_resample", title: "Pattern Resample", desc: "Pattern resampling with various filter types." },
  { route: "lion_outline", title: "Lion Outline", desc: "Lion rendered as stroked outlines with adjustable width." },
  { route: "rasterizers2", title: "Rasterizers 2", desc: "Extended rasterizer comparison with outline and gamma controls." },
  { route: "line_patterns", title: "Line Patterns", desc: "Custom line patterns with clip regions." },
  { route: "line_patterns_clip", title: "Line Patterns Clip", desc: "Line patterns with clipping rectangle." },
  { route: "compositing", title: "Compositing", desc: "Porter-Duff compositing operators visualization." },
  { route: "compositing2", title: "Compositing 2", desc: "Advanced compositing with alpha blending modes." },
  { route: "flash_rasterizer", title: "Flash Rasterizer", desc: "Flash-style compound shape rasterization." },
  { route: "flash_rasterizer2", title: "Flash Rasterizer 2", desc: "Extended Flash-style rasterizer with styles." },
  { route: "rasterizer_compound", title: "Compound Rasterizer", desc: "Compound shape rasterizer with style handler." },
  { route: "gouraud_mesh", title: "Gouraud Mesh", desc: "Triangle mesh with Gouraud shading interpolation." },
  { route: "image_resample", title: "Image Resample", desc: "Image resampling with perspective transform and filter comparison." },
  { route: "alpha_mask2", title: "Alpha Mask 2", desc: "Alpha mask with gray8 rendering buffer." },
  { route: "truetype_test", title: "TrueType LCD", desc: "LCD subpixel font rendering with faux weight, faux italic, gamma, and multiple typefaces." }
];
function renderHome(container) {
  const cardsHtml = demoCards.map((card) => `
        <a href="#/${card.route}" class="feature-card">
          ${thumbImg(card.route, "card-thumb")}
          <h3>${card.title}</h3>
          <p>${card.desc}</p>
        </a>`).join("");
  container.innerHTML = `
    <div class="home-page">
      <div class="github-badge">
        <a href="https://github.com/larsbrubaker/agg-rust" target="_blank" class="github-badge-link">
          <svg height="20" viewBox="0 0 16 16" width="20" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
          <span>larsbrubaker/agg-rust</span>
        </a>
      </div>
      <div class="hero">
        <h1>AGG <span>for Rust</span></h1>
        <p>
          A pure Rust port of Anti-Grain Geometry (AGG) 2.6 &mdash; the legendary
          high-quality 2D software rendering library. Explore interactive demos
          showcasing anti-aliased rendering, gradient fills, Gouraud shading,
          and more &mdash; all running in your browser via WebAssembly.
        </p>
      </div>
      <div class="feature-grid">${cardsHtml}
      </div>
      <div class="about-section">
        <h2>About This Project</h2>
        <p>
          This is a pure Rust port of Maxim Shemanarev's
          <a href="http://www.antigrain.com" target="_blank">Anti-Grain Geometry</a>
          C++ library (version 2.6). AGG is a software rendering engine that
          produces pixel-perfect anti-aliased output without relying on any
          GPU or platform graphics API.
        </p>
        <p style="margin-top: 12px">
          Ported by <strong>Lars Brubaker</strong>, sponsored by
          <a href="https://www.matterhackers.com" target="_blank">MatterHackers</a>.
        </p>
        <div class="stats-row">
          <div class="stat">
            <div class="stat-value">88</div>
            <div class="stat-label">Modules Ported</div>
          </div>
          <div class="stat">
            <div class="stat-value">903</div>
            <div class="stat-label">Tests Passing</div>
          </div>
          <div class="stat">
            <div class="stat-value">100%</div>
            <div class="stat-label">Software Rendered</div>
          </div>
          <div class="stat">
            <div class="stat-value">0</div>
            <div class="stat-label">GPU Dependencies</div>
          </div>
        </div>
      </div>
    </div>
  `;
}
async function navigate(route) {
  const container = document.getElementById("main-content");
  const canonicalRouteKey = canonicalRoute(route);
  saveCurrentRouteScroll();
  if (currentCleanup) {
    currentCleanup();
    currentCleanup = null;
  }
  updateNav(canonicalRouteKey);
  if (canonicalRouteKey === "home") {
    renderHome(container);
    finalizeNavigation(canonicalRouteKey);
    return;
  }
  if (canonicalRouteKey === "history" || canonicalRouteKey.startsWith("history/") || route === "legacy" || route.startsWith("legacy/")) {
    await renderHistoryRoute(container, canonicalRouteKey);
    finalizeNavigation(canonicalRouteKey);
    return;
  }
  const loader = demoModules[canonicalRouteKey];
  if (!loader) {
    container.innerHTML = `<div class="home-page"><h2>Page not found</h2><p>Unknown route: ${canonicalRouteKey}</p></div>`;
    finalizeNavigation(canonicalRouteKey);
    return;
  }
  container.innerHTML = `<div class="home-page" style="display:flex;align-items:center;justify-content:center;height:80vh;"><p style="color:var(--text-muted)">Loading demo...</p></div>`;
  try {
    await initWasm();
    const mod = await loader();
    container.innerHTML = "";
    const cleanup = mod.init(container);
    if (cleanup)
      currentCleanup = cleanup;
    finalizeNavigation(canonicalRouteKey);
  } catch (e) {
    console.error("Failed to load demo:", e);
    container.innerHTML = `<div class="home-page"><h2>Error loading demo</h2><pre style="color:var(--accent)">${e}</pre></div>`;
    finalizeNavigation(canonicalRouteKey);
  }
}
window.addEventListener("hashchange", () => navigate(getRoute()));
navigate(getRoute());

//# debugId=24505BE1DBED4F1264756E2164756E21
