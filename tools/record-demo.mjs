// Grava o modo demonstração da interface (docs/media) para o README.
// Uso: servir ui/ em :8765 e rodar `NODE_PATH=<dir com playwright-core> node tools/record-demo.mjs`.
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright-core');

const OUT = new URL('../docs/media/', import.meta.url).pathname;
const URL_DEMO = 'http://127.0.0.1:8765/?demo=1';
const size = { width: 1180, height: 720 };
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await chromium.launch(process.env.CHROME ? { executablePath: process.env.CHROME } : {});
const ctx = await browser.newContext({ viewport: size, recordVideo: { dir: OUT, size } });
const page = await ctx.newPage();
await page.goto(URL_DEMO);
await page.addStyleTag({ content: '#conn{display:none}' });
await wait(3200);
await page.screenshot({ path: OUT + 'inicio.png' });
const label = (t) => page.locator('.nlabel', { hasText: t });
await label('Turbo').click();
await wait(2600);
await page.screenshot({ path: OUT + 'turbo.png' });
await label('Eco').click();
await wait(2300);
await page.locator('.dcenter').click();
await wait(2300);
for (const [tab, shot, ms] of [['keyboard', 'teclado', 1800], ['monitor', 'monitor', 1800], ['system', 'sistema', 1500]]) {
  await page.locator(`#tabs button[data-tab=${tab}]`).click();
  await wait(ms);
  await page.screenshot({ path: `${OUT}${shot}.png` });
}
await page.locator('#tabs button[data-tab=home]').click();
await wait(1200);
await label('Desempenho').click();
await wait(2400);
const video = page.video();
await ctx.close();
await browser.close();
console.log(await video.path());
