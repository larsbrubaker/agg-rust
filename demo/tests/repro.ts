// Empirical reproduction for canvas drag/click bugs.
// Loads each demo, performs a drag gesture at a candidate location,
// and reports whether the rendered canvas changed (i.e. the interaction worked).
//
// Start server first (PORT=3111 bun run server.ts), then:
//   CHROME_EXE=./.chrome/chrome-headless-shell.exe PORT=3111 bun run tests/repro.ts

import { chromium } from 'playwright';
import { appendFileSync, writeFileSync } from 'fs';

const BASE = process.env.BASE || 'http://localhost:3111';
const LOG = new URL('./repro.out.txt', import.meta.url).pathname.replace(/^\//, '');
function log(s: string) { console.log(s); try { appendFileSync(LOG, s + '\n'); } catch {} }
writeFileSync(LOG, '');

interface Probe { name: string; route: string; aggX: number; aggY: number; dx: number; dy: number; }

const probes: Probe[] = [
  { name: 'aa_demo vertex[0]', route: 'aa_demo', aggX: 57, aggY: 100, dx: 40, dy: -30 },
  { name: 'line_patterns point[0]', route: 'line_patterns', aggX: 64, aggY: 19, dx: 40, dy: 40 },
  { name: 'gradients center', route: 'gradients', aggX: 350, aggY: 280, dx: -60, dy: 40 },
];

async function canvasHash(page: any): Promise<string> {
  return await page.evaluate(() => {
    const c = document.getElementById('demo-canvas') as HTMLCanvasElement;
    if (!c) return 'NO-CANVAS';
    const ctx = c.getContext('2d')!;
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let h = 0;
    for (let i = 0; i < d.length; i += 97) { h = (h * 31 + d[i]) | 0; }
    return String(h) + ':' + c.width + 'x' + c.height;
  });
}

async function rectInfo(page: any) {
  return await page.evaluate(() => {
    const c = document.getElementById('demo-canvas') as HTMLCanvasElement;
    const r = c.getBoundingClientRect();
    return { left: r.left, top: r.top, width: r.width, height: r.height, cw: c.width, ch: c.height, transform: getComputedStyle(c).transform };
  });
}

async function tryDrag(page: any, startX: number, startY: number, dx: number, dy: number) {
  const before = await canvasHash(page);
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + dx * 0.5, startY + dy * 0.5, { steps: 4 });
  await page.mouse.move(startX + dx, startY + dy, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(150);
  const after = await canvasHash(page);
  return { changed: before !== after, before, after };
}

async function loadDemo(page: any, route: string) {
  await page.goto(`${BASE}/#/${route}`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('#demo-canvas', { timeout: 15000 });
  // wait until canvas actually has rendered content (non-uniform)
  await page.waitForFunction(() => {
    const c = document.getElementById('demo-canvas') as HTMLCanvasElement;
    return c && c.width > 0 && c.height > 0;
  }, { timeout: 15000 });
  await page.waitForTimeout(700);
}

async function main() {
  const exe = process.env.CHROME_EXE;
  log('launching browser exe=' + (exe || 'default'));
  const browser = await chromium.launch(exe ? { executablePath: exe } : {});
  const page = await browser.newPage({ viewport: { width: 1500, height: 1100 } });
  page.on('console', (m: any) => { if (m.type() === 'error') log('  [page error] ' + m.text()); });
  page.on('pageerror', (e: any) => log('  [pageerror] ' + e.message));

  for (const p of probes) {
    try {
      // FLIPPED hypothesis: feature displayed at screen row (ch - aggY)
      await loadDemo(page, p.route);
      const ri = await rectInfo(page);
      const fx = ri.left + p.aggX * (ri.width / ri.cw);
      const fy = ri.top + (ri.ch - p.aggY) * (ri.height / ri.ch);
      const rFlip = await tryDrag(page, fx, fy, p.dx, p.dy);

      // NOFLIP hypothesis: feature displayed at screen row aggY
      await loadDemo(page, p.route);
      const nx = ri.left + p.aggX * (ri.width / ri.cw);
      const ny = ri.top + p.aggY * (ri.height / ri.ch);
      const rNo = await tryDrag(page, nx, ny, p.dx, p.dy);

      log(`\n== ${p.name} (${p.route}) canvas ${ri.cw}x${ri.ch} transform=${ri.transform}`);
      log(`   FLIPPED screen(${fx.toFixed(0)},${fy.toFixed(0)}) changed=${rFlip.changed}`);
      log(`   NOFLIP  screen(${nx.toFixed(0)},${ny.toFixed(0)}) changed=${rNo.changed}`);
    } catch (e: any) {
      log(`\n== ${p.name} (${p.route}) ERROR ${e.message}`);
    }
  }

  await browser.close();
  log('\nDONE');
}

main().catch(e => { log('FATAL ' + e.message + '\n' + e.stack); process.exit(1); });
