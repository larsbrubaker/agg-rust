// Reusable mouse interaction helpers for AGG demos.
// Uses pointer events with setPointerCapture so drags work even when
// the cursor leaves the canvas.

export interface Vertex {
  x: number;
  y: number;
}

/** Get pointer position relative to canvas in AGG coordinates (origin bottom-left). */
function canvasPos(canvas: HTMLCanvasElement, e: PointerEvent): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: canvas.height - (e.clientY - rect.top) * scaleY,
  };
}

// ============================================================================
// Pattern 1: Vertex Dragging
// ============================================================================

export interface VertexDragOptions {
  canvas: HTMLCanvasElement;
  vertices: Vertex[];
  threshold?: number;       // grab radius in pixels (default 10)
  dragEdges?: boolean;      // allow dragging polygon edges (moves 2 adjacent vertices)
  edgeThreshold?: number;   // edge grab distance in pixels (default = threshold)
  dragAll?: boolean;        // allow dragging all vertices when clicking inside shape
  dragAllHitTest?: (x: number, y: number, vertices: Vertex[]) => boolean;
  onDrag: () => void;       // called whenever vertices change
}

/**
 * Set up vertex dragging on a canvas. Returns a cleanup function.
 * Left-click near a vertex to grab it, drag to move, release to drop.
 */
export function setupVertexDrag(opts: VertexDragOptions): () => void {
  const { canvas, vertices, onDrag } = opts;
  const threshold = opts.threshold ?? 10;
  const edgeThreshold = opts.edgeThreshold ?? threshold;
  let dragIdx = -1;
  let dragEdge = -1;
  let dx = 0, dy = 0;

  function distanceToSegment(
    px: number, py: number,
    x1: number, y1: number,
    x2: number, y2: number,
  ): { distance: number; t: number } {
    const vx = x2 - x1;
    const vy = y2 - y1;
    const len2 = vx * vx + vy * vy;
    if (len2 <= 1e-12) {
      return { distance: Math.hypot(px - x1, py - y1), t: 0 };
    }
    const tRaw = ((px - x1) * vx + (py - y1) * vy) / len2;
    const t = Math.max(0, Math.min(1, tRaw));
    const cx = x1 + t * vx;
    const cy = y1 + t * vy;
    return { distance: Math.hypot(px - cx, py - cy), t };
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = canvasPos(canvas, e);

    // Find nearest vertex within threshold
    let bestDist = threshold;
    let bestIdx = -1;
    for (let i = 0; i < vertices.length; i++) {
      const d = Math.hypot(pos.x - vertices[i].x, pos.y - vertices[i].y);
      if (d < bestDist) { bestDist = d; bestIdx = i; }
    }

    if (bestIdx >= 0) {
      dragIdx = bestIdx;
      dragEdge = -1;
      dx = pos.x - vertices[bestIdx].x;
      dy = pos.y - vertices[bestIdx].y;
      canvas.setPointerCapture(e.pointerId);
    } else if (opts.dragEdges && vertices.length >= 2) {
      let bestEdgeDist = edgeThreshold;
      let bestEdge = -1;
      const n = vertices.length;
      for (let i = 0; i < n; i++) {
        // Match AGG interactive_polygon: edge i is between i and (i+n-1)%n
        const n1 = i;
        const n2 = (i + n - 1) % n;
        const hit = distanceToSegment(
          pos.x, pos.y,
          vertices[n1].x, vertices[n1].y,
          vertices[n2].x, vertices[n2].y,
        );
        if (hit.t > 0 && hit.t < 1 && hit.distance < bestEdgeDist) {
          bestEdgeDist = hit.distance;
          bestEdge = i;
        }
      }
      if (bestEdge >= 0) {
        dragIdx = -1;
        dragEdge = bestEdge;
        dx = pos.x;
        dy = pos.y;
        canvas.setPointerCapture(e.pointerId);
      }
    } else if (opts.dragAll && vertices.length >= 3) {
      const canDragAll = opts.dragAllHitTest
        ? opts.dragAllHitTest(pos.x, pos.y, vertices)
        : true;
      if (canDragAll) {
        // Drag all vertices together (click inside shape)
        dragIdx = vertices.length; // special index
        dragEdge = -1;
        dx = pos.x - vertices[0].x;
        dy = pos.y - vertices[0].y;
        canvas.setPointerCapture(e.pointerId);
      }
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (dragIdx < 0 && dragEdge < 0) return;
    const pos = canvasPos(canvas, e);

    if (dragIdx < vertices.length) {
      vertices[dragIdx].x = pos.x - dx;
      vertices[dragIdx].y = pos.y - dy;
    } else if (dragEdge >= 0 && vertices.length >= 2) {
      const n = vertices.length;
      const n1 = dragEdge;
      const n2 = (n1 + n - 1) % n;
      const ddx = pos.x - dx;
      const ddy = pos.y - dy;
      vertices[n1].x += ddx;
      vertices[n1].y += ddy;
      vertices[n2].x += ddx;
      vertices[n2].y += ddy;
      dx = pos.x;
      dy = pos.y;
    } else {
      // Drag all
      const newX = pos.x - dx;
      const newY = pos.y - dy;
      const ddx = newX - vertices[0].x;
      const ddy = newY - vertices[0].y;
      for (const v of vertices) { v.x += ddx; v.y += ddy; }
    }
    onDrag();
  }

  function onPointerUp() {
    dragIdx = -1;
    dragEdge = -1;
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}

// ============================================================================
// Pattern 2: Rotate/Scale from Center (lion, gradients)
// ============================================================================

export interface RotateScaleOptions {
  canvas: HTMLCanvasElement;
  /** Called on left-button drag with angle (radians) and scale factor. */
  onLeftDrag: (angle: number, scale: number) => void;
  /** Called on right-button drag with (x, y) position. */
  onRightDrag?: (x: number, y: number) => void;
}

/**
 * Set up rotate/scale interaction on a canvas. Returns a cleanup function.
 * Left-drag: angle = atan2(dy, dx), scale = dist / 100 (matching C++ lion.cpp)
 * Right-drag: skew_x = x, skew_y = y
 */
export function setupRotateScale(opts: RotateScaleOptions): () => void {
  const { canvas, onLeftDrag, onRightDrag } = opts;

  function onPointerDown(e: PointerEvent) {
    canvas.setPointerCapture(e.pointerId);
    handlePointer(e);
  }

  function onPointerMove(e: PointerEvent) {
    if (e.buttons === 0) return;
    handlePointer(e);
  }

  function handlePointer(e: PointerEvent) {
    const pos = canvasPos(canvas, e);
    const cx = canvas.width / 2;
    const cy = canvas.height / 2;
    const dx = pos.x - cx;
    const dy = pos.y - cy;

    if (e.buttons & 1) { // left button
      const angle = Math.atan2(dy, dx);
      const scale = Math.hypot(dx, dy) / 100.0;
      onLeftDrag(angle, scale);
    }

    if ((e.buttons & 2) && onRightDrag) { // right button
      onRightDrag(pos.x, pos.y);
    }
  }

  function onContextMenu(e: Event) {
    e.preventDefault();
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('contextmenu', onContextMenu);

  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}

// ============================================================================
// Pattern 3: Gradient-style drag (translate center + rotate/scale)
// ============================================================================

export interface GradientDragOptions {
  canvas: HTMLCanvasElement;
  centerX: number;
  centerY: number;
  angle: number;
  scale: number;
  onUpdate: (cx: number, cy: number, angle: number, scale: number) => void;
}

/**
 * Set up gradient-style mouse interaction.
 * Left-drag: translate center position.
 * Right-drag: rotate and scale (relative to initial grab).
 */
export function setupGradientDrag(opts: GradientDragOptions): () => void {
  const { canvas, onUpdate } = opts;
  let cx = opts.centerX, cy = opts.centerY;
  let angle = opts.angle, scale = opts.scale;
  let pdx = 0, pdy = 0;
  let prevAngle = 0, prevScale = 1;

  function onPointerDown(e: PointerEvent) {
    canvas.setPointerCapture(e.pointerId);
    const pos = canvasPos(canvas, e);
    pdx = cx - pos.x;
    pdy = cy - pos.y;
    prevScale = scale;
    prevAngle = angle + Math.PI;
  }

  function onPointerMove(e: PointerEvent) {
    if (e.buttons === 0) return;
    const pos = canvasPos(canvas, e);

    if (e.buttons & 1) { // left: translate
      cx = pos.x + pdx;
      cy = pos.y + pdy;
    }

    if (e.buttons & 2) { // right: rotate + scale
      const dx = pos.x - cx;
      const dy = pos.y - cy;
      const dist = Math.hypot(dx, dy);
      const prevDist = Math.hypot(pdx, pdy);
      if (prevDist > 1) {
        scale = prevScale * dist / prevDist;
      }
      angle = prevAngle + Math.atan2(dy, dx) - Math.atan2(pdy, pdx);
    }

    onUpdate(cx, cy, angle, scale);
  }

  function onContextMenu(e: Event) { e.preventDefault(); }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('contextmenu', onContextMenu);

  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}
