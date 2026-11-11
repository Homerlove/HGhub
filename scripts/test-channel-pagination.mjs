// ChannelPage pagination regression: page 2 must stay after page 1, and sort must be page-local.
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { chromium } from "playwright-core";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "test-results", "channel-pagination", new Date().toISOString().replace(/[:.]/g, "-"));
const report = {
  transport: "local browser fixture; Tauri IPC mocked",
  passed: [],
  external: [],
  pageErrors: [],
};
await mkdir(output, { recursive: true });
let vite, browser, page;
try {
  vite = await createServer({ root, server: { host: "127.0.0.1", port: 0, strictPort: false } });
  await vite.listen();
  const origin = vite.resolvedUrls.local[0];
  browser = await chromium.launch({ headless: true, executablePath: process.env.BROWSER_EXECUTABLE || undefined });
  page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", error => report.pageErrors.push(error.message));
  await page.route("**/*", route => {
    const url = route.request().url();
    if (new URL(url).origin === new URL(origin).origin) return route.continue();
    report.external.push(url);
    return route.abort();
  });

  await page.goto(`${origin}tests/channel-pagination.html`);
  await page.waitForFunction(() => window.channelPaginationFixture.history.length === 1);
  await page.evaluate(() => window.channelPaginationFixture.resolveNext());
  await page.waitForFunction(() => document.querySelectorAll(".grid > div .clamp-2").length === 20);

  const fixture = await page.evaluate(() => ({
    page1: window.channelPaginationFixture.page1,
    page2: window.channelPaginationFixture.page2,
  }));
  const titles = async () => page.locator(".grid > div .clamp-2").allInnerTexts();
  const expectedPage = (list, key) => [...list]
    .sort((a, b) => key === "hot" ? (b.hot - a.hot) : key === "score" ? (b.score - a.score) : (b.id - a.id))
    .map(v => v.title);
  const scroller = page.locator(".scroll-y").last();
  const before = await titles();
  assert.equal(before.length, 20, "First page must show 20 cards");
  assert.deepEqual(before, expectedPage(fixture.page1, "id"), "First page must be sorted by id within page 1");

  await scroller.evaluate(el => { el.scrollTop = el.scrollHeight; });
  await page.waitForFunction(() => window.channelPaginationFixture.history.length === 2);
  const scrollBefore = await scroller.evaluate(el => ({ top: el.scrollTop, height: el.clientHeight, scrollHeight: el.scrollHeight }));
  await page.evaluate(() => window.channelPaginationFixture.resolveNext());
  await page.waitForFunction(() => document.querySelectorAll(".grid > div .clamp-2").length === 40);
  await page.waitForTimeout(250);

  const after = await titles();
  const scrollAfter = await scroller.evaluate(el => ({ top: el.scrollTop, height: el.clientHeight, scrollHeight: el.scrollHeight }));
  assert.equal(after.length, 40, "Second page must append all cards");
  assert.deepEqual(after.slice(0, 20), before, "All page 1 cards must remain above page 2 cards");
  assert.deepEqual(after.slice(20), expectedPage(fixture.page2, "id"), "Page 2 cards must stay in page-local id order");
  assert(scrollAfter.top > 0 && scrollAfter.top >= scrollBefore.top - 1, "Loading page 2 must not reset scrollTop");
  report.passed.push("ChannelPage: page 2 appended after page 1 and scrollTop preserved");

  await page.getByRole("button", { name: "最热", exact: true }).click();
  await page.waitForTimeout(120);
  assert.deepEqual(await titles(), [...expectedPage(fixture.page1, "hot"), ...expectedPage(fixture.page2, "hot")], "最热 must sort within each page only");
  report.passed.push("ChannelPage: 最热 is page-local");

  await page.getByRole("button", { name: "高分", exact: true }).click();
  await page.waitForTimeout(120);
  assert.deepEqual(await titles(), [...expectedPage(fixture.page1, "score"), ...expectedPage(fixture.page2, "score")], "高分 must sort within each page only");
  report.passed.push("ChannelPage: 高分 is page-local");

  assert.deepEqual(report.pageErrors, [], "No page errors");
  assert.deepEqual(report.external, [], "No upstream traffic");
  await page.screenshot({ path: join(output, "final.png"), fullPage: true });
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  report.error = String(error.stack || error);
  await page?.screenshot({ path: join(output, "failure.png"), fullPage: true }).catch(() => {});
  console.error(report.error);
  process.exitCode = 1;
} finally {
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2));
  await browser?.close();
  await vite?.close();
  console.log(`Artifacts: ${output}`);
}
