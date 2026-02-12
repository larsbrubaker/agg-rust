// WASM module loader for AGG-Rust demos

let wasmModule: any = null;

export async function initWasm(): Promise<void> {
  if (wasmModule) return;
  const wasmUrl = new URL('./public/pkg/agg_wasm.js', window.location.href).href;
  const mod = await import(wasmUrl);
  await mod.default();
  wasmModule = mod;
}

function getWasm(): any {
  if (!wasmModule) throw new Error('WASM not initialized. Call initWasm() first.');
  return wasmModule;
}

/**
 * Render a named demo and return RGBA pixel data.
 */
export function renderDemo(name: string, width: number, height: number, params: number[]): Uint8Array {
  const w = getWasm();
  const result = w.render_demo(name, width, height, new Float64Array(params));
  return new Uint8Array(result);
}

export function flashPickVertex(
  demoName: 'flash_rasterizer' | 'flash_rasterizer2',
  width: number,
  height: number,
  params: number[],
  x: number,
  y: number,
  radius: number,
): number {
  const w = getWasm();
  return w.flash_pick_vertex(demoName, width, height, new Float64Array(params), x, y, radius);
}

export function flashScreenToShape(
  demoName: 'flash_rasterizer' | 'flash_rasterizer2',
  width: number,
  height: number,
  params: number[],
  x: number,
  y: number,
): [number, number] {
  const w = getWasm();
  const out = w.flash_screen_to_shape(demoName, width, height, new Float64Array(params), x, y) as number[] | Float64Array;
  return [out[0] ?? x, out[1] ?? y];
}

export function gouraudMeshPickVertex(
  width: number,
  height: number,
  params: number[],
  x: number,
  y: number,
  radius: number,
): number {
  const w = getWasm();
  return w.gouraud_mesh_pick_vertex(width, height, new Float64Array(params), x, y, radius);
}

/**
 * Get AGG-Rust version.
 */
export function getVersion(): string {
  return getWasm().version();
}
