// Dev server — serves the demo site locally
// Usage: bun run server.ts

import { join } from 'path';

const PORT = 3000;
const ROOT = import.meta.dir;

const MIME_TYPES: Record<string, string> = {
  '.html': 'text/html',
  '.css': 'text/css',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.txt': 'text/plain',
  '.map': 'application/json',
};

const server = Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    let pathname = decodeURIComponent(url.pathname);

    if (pathname === '/') pathname = '/index.html';

    const filePath = join(ROOT, pathname);
    const file = Bun.file(filePath);

    if (await file.exists()) {
      const ext = pathname.substring(pathname.lastIndexOf('.'));
      const contentType = MIME_TYPES[ext] || 'application/octet-stream';
      return new Response(file, {
        headers: {
          'Content-Type': contentType,
          'Access-Control-Allow-Origin': '*',
        },
      });
    }

    return new Response('Not found', { status: 404 });
  },
});

console.log(`AGG-Rust demos running at http://localhost:${server.port}`);
console.log(`Press Ctrl+C to stop.`);
