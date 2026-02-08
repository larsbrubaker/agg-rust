// Dev server with watch mode — rebuilds WASM + TS on file changes
// Usage: bun run server.ts

import { join, extname } from "path";
import { watch } from "fs";

const PORT = 3000;
const ROOT = join(import.meta.dir, "public");
const PROJECT_ROOT = join(import.meta.dir, "..");

const MIME_TYPES: Record<string, string> = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain",
  ".map": "application/json",
};

// --- Live reload via SSE ---
const reloadClients = new Set<ReadableStreamDefaultController>();

const RELOAD_SCRIPT = `<script>
(function(){
  const es = new EventSource("/__reload");
  es.onmessage = function(e) {
    if (e.data === "reload") location.reload();
  };
})();
</script>`;

// --- File serving ---
async function serveFile(pathname: string): Promise<Response | null> {
  const filePath = join(ROOT, pathname);
  const file = Bun.file(filePath);
  if (!(await file.exists())) return null;

  const ext = extname(pathname);
  const contentType = MIME_TYPES[ext] || "application/octet-stream";

  // Inject live-reload script into HTML
  if (ext === ".html") {
    let html = await file.text();
    html = html.replace("</body>", `${RELOAD_SCRIPT}</body>`);
    return new Response(html, {
      headers: {
        "Content-Type": "text/html",
        "Access-Control-Allow-Origin": "*",
      },
    });
  }

  return new Response(file, {
    headers: {
      "Content-Type": contentType,
      "Access-Control-Allow-Origin": "*",
    },
  });
}

const server = Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    let pathname = decodeURIComponent(url.pathname);

    // SSE endpoint for live reload
    if (pathname === "/__reload") {
      const stream = new ReadableStream({
        start(controller) {
          reloadClients.add(controller);
          controller.enqueue("data: connected\n\n");
        },
        cancel(controller) {
          reloadClients.delete(controller);
        },
      });
      return new Response(stream, {
        headers: {
          "Content-Type": "text/event-stream",
          "Cache-Control": "no-cache",
          Connection: "keep-alive",
          "Access-Control-Allow-Origin": "*",
        },
      });
    }

    if (pathname === "/") pathname = "/index.html";

    // Try public/ first, then demo root for index.html
    const resp = await serveFile(pathname);
    if (resp) return resp;

    // Fallback: serve from demo root (for index.html at demo level)
    const demoFile = Bun.file(join(import.meta.dir, pathname));
    if (await demoFile.exists()) {
      const ext = extname(pathname);
      const contentType = MIME_TYPES[ext] || "application/octet-stream";
      if (ext === ".html") {
        let html = await demoFile.text();
        html = html.replace("</body>", `${RELOAD_SCRIPT}</body>`);
        return new Response(html, {
          headers: { "Content-Type": "text/html", "Access-Control-Allow-Origin": "*" },
        });
      }
      return new Response(demoFile, {
        headers: { "Content-Type": contentType, "Access-Control-Allow-Origin": "*" },
      });
    }

    return new Response("Not found", { status: 404 });
  },
});

function notifyReload() {
  for (const controller of reloadClients) {
    try {
      controller.enqueue("data: reload\n\n");
    } catch {
      reloadClients.delete(controller);
    }
  }
}

// --- Rebuild helpers ---
let building = false;
let pendingRust = false;
let pendingTs = false;

async function runCommand(cmd: string[], cwd: string, label: string): Promise<boolean> {
  console.log(`\x1b[36m[${label}]\x1b[0m Building...`);
  const start = Date.now();
  const proc = Bun.spawn(cmd, { cwd, stdout: "inherit", stderr: "inherit" });
  const code = await proc.exited;
  const elapsed = ((Date.now() - start) / 1000).toFixed(1);
  if (code === 0) {
    console.log(`\x1b[32m[${label}]\x1b[0m Done in ${elapsed}s`);
    return true;
  } else {
    console.error(`\x1b[31m[${label}]\x1b[0m Failed (exit ${code})`);
    return false;
  }
}

async function buildWasm(): Promise<boolean> {
  return runCommand(
    [
      "wasm-pack",
      "build",
      "demo/wasm",
      "--target",
      "web",
      "--out-dir",
      "../public/pkg",
      "--no-typescript",
    ],
    PROJECT_ROOT,
    "wasm"
  );
}

async function buildTs(): Promise<boolean> {
  return runCommand(["bun", "run", "build.ts"], import.meta.dir, "ts");
}

async function rebuild() {
  if (building) return;
  building = true;

  while (pendingRust || pendingTs) {
    const doRust = pendingRust;
    const doTs = pendingTs;
    pendingRust = false;
    pendingTs = false;

    let changed = false;
    if (doRust) {
      const ok = await buildWasm();
      if (ok) changed = true;
    }
    if (doTs || doRust) {
      // Rust changes affect WASM which TS imports, so rebuild TS too
      const ok = await buildTs();
      if (ok) changed = true;
    }
    if (changed) notifyReload();
  }

  building = false;
}

// --- File watchers ---
function debounce(fn: () => void, ms: number) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(fn, ms);
  };
}

const triggerRustBuild = debounce(() => {
  pendingRust = true;
  rebuild();
}, 300);

const triggerTsBuild = debounce(() => {
  pendingTs = true;
  rebuild();
}, 200);

// Watch Rust source files (main crate + wasm crate)
for (const dir of [join(PROJECT_ROOT, "src"), join(import.meta.dir, "wasm", "src")]) {
  watch(dir, { recursive: true }, (event, filename) => {
    if (filename && filename.endsWith(".rs")) {
      console.log(`\x1b[33m[watch]\x1b[0m Rust changed: ${filename}`);
      triggerRustBuild();
    }
  });
}

// Watch TypeScript demo source
watch(join(import.meta.dir, "src"), { recursive: true }, (event, filename) => {
  if (filename && (filename.endsWith(".ts") || filename.endsWith(".tsx"))) {
    console.log(`\x1b[33m[watch]\x1b[0m TS changed: ${filename}`);
    triggerTsBuild();
  }
});

// Watch HTML/CSS in demo root
for (const dir of [import.meta.dir, join(import.meta.dir, "styles")]) {
  watch(dir, { recursive: false }, (event, filename) => {
    if (filename && (filename.endsWith(".html") || filename.endsWith(".css"))) {
      console.log(`\x1b[33m[watch]\x1b[0m Static changed: ${filename}`);
      notifyReload();
    }
  });
}

console.log(`\x1b[32mAGG-Rust demos running at http://localhost:${server.port}\x1b[0m`);
console.log(`Watching for changes in:`);
console.log(`  Rust:  src/**/*.rs, demo/wasm/src/**/*.rs`);
console.log(`  TS:    demo/src/**/*.ts`);
console.log(`  Static: demo/*.html, demo/styles/*.css`);
console.log(`Press Ctrl+C to stop.`);
