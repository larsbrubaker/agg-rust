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

/**
 * Get AGG-Rust version.
 */
export function getVersion(): string {
  return getWasm().version();
}
