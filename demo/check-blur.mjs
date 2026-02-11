import { chromium } from 'playwright';

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];

page.on('console', (m) => {
  if (m.type() === 'error' || m.type() === 'warning') errors.push({ type: m.type(), text: m.text() });
});

await page.goto('http://localhost:3000/#/blur', { waitUntil: 'networkidle', timeout: 15000 });
await page.waitForTimeout(2000);

const canvas = await page.$('canvas#demo-canvas');
const info = canvas ? await canvas.evaluate((c) => ({ w: c.width, h: c.height })) : null;
const blank = canvas
  ? await canvas.evaluate((c) => {
      const ctx = c.getContext('2d');
      if (!ctx) return 'no-ctx';
      const d = ctx.getImageData(0, 0, Math.min(200, c.width), Math.min(200, c.height)).data;
      let n = 0;
      for (let i = 0; i < d.length; i += 4) if (d[i] || d[i + 1] || d[i + 2] || d[i + 3]) n++;
      return n === 0 ? 'blank' : 'has-content';
    })
  : 'no-canvas';

const timeEl = await page.$('#render-time');
const timeText = timeEl ? await timeEl.textContent() : '';

// Try sidebar: change radius slider
const radiusSlider = await page.$('#demo-sidebar input[type="range"]');
let radiusWorks = false;
if (radiusSlider) {
  await radiusSlider.fill('5');
  await page.waitForTimeout(600);
  const t1 = await page.$('#render-time');
  const txt1 = t1 ? await t1.textContent() : '';
  const c2 = await page.$('canvas#demo-canvas');
  const hasContent2 = c2
    ? await c2.evaluate((c) => {
        const ctx = c.getContext('2d');
        if (!ctx) return false;
        const d = ctx.getImageData(0, 0, Math.min(50, c.width), Math.min(50, c.height)).data;
        for (let i = 0; i < d.length; i += 4) if (d[i] || d[i + 1] || d[i + 2] || d[i + 3]) return true;
        return false;
      })
    : false;
  radiusWorks = txt1 !== 'render failed' && hasContent2;
}

// Try method radio (Recursive blur)
const radio1 = await page.$('input[type="radio"][value="1"]');
let methodWorks = false;
if (radio1) {
  await radio1.click();
  await page.waitForTimeout(600);
  const t2 = await page.$('#render-time');
  methodWorks = t2 ? (await t2.textContent()) !== 'render failed' : false;
}

// Sample some pixels for basic sanity (center region)
let samplePixels = null;
if (canvas && blank === 'has-content') {
  samplePixels = await canvas.evaluate((c) => {
    const ctx = c.getContext('2d');
    const d = ctx.getImageData(Math.floor(c.width / 2) - 5, Math.floor(c.height / 2) - 5, 10, 10).data;
    return [d[0], d[1], d[2], d[4], d[5], d[6]];
  });
}

console.log(
  JSON.stringify(
    {
      renders: blank === 'has-content',
      timeText,
      canvasSize: info,
      sidebarRadiusWorks: radiusWorks,
      sidebarMethodWorks: methodWorks,
      samplePixels,
      errors: errors.slice(0, 3),
    },
    null,
    2,
  ),
);

await browser.close();
