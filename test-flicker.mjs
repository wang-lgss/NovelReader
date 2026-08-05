// Playwright: analyze flickering causes
import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: false });
const page = await browser.newPage();

// Collect all console messages
const logs = [];
page.on('console', msg => logs.push({ type: msg.type(), text: msg.text() }));

// Monitor layout thrashing: intercept forced reflows
await page.evaluate(() => {
  const origGetScrollTop = Object.getOwnPropertyDescriptor(Element.prototype, 'scrollTop').get;
  const origClientHeight = Object.getOwnPropertyDescriptor(Element.prototype, 'clientHeight').get;
  const origScrollHeight = Object.getOwnPropertyDescriptor(Element.prototype, 'scrollHeight').get;

  let layoutCount = 0;
  let lastCaller = '';

  Object.defineProperty(Element.prototype, 'scrollTop', {
    get() {
      layoutCount++;
      return origGetScrollTop.call(this);
    },
    set(v) { Object.defineProperty(this, 'scrollTop', { value: v, writable: true }); this.scrollTop = v; }
  });

  // Log layout reads per second
  setInterval(() => {
    if (layoutCount > 100) {
      console.warn(`[LAYOUT] ${layoutCount} forced reflows in 1s`);
    }
    layoutCount = 0;
  }, 1000);
});

await page.goto('http://localhost:2048', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('#reader:not([style*="display: none"])', { timeout: 10000 });

console.log('=== App loaded ===');

// Check for JS errors
const errors = logs.filter(l => l.type === 'error');
if (errors.length > 0) {
  console.log('JS errors found:');
  errors.forEach(e => console.log(`  ${e.text}`));
} else {
  console.log('No JS errors');
}

// Check for layout warnings
const warns = logs.filter(l => l.type === 'warning');
if (warns.length > 0) {
  console.log('Layout warnings:');
  warns.forEach(w => console.log(`  ${w.text}`));
}

// Do a few chapter navigations via wheel (scroll trigger)
const area = page.locator('#contentArea');
const box = await area.boundingBox();
await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);

// Scroll to bottom and trigger next chapter
await page.evaluate(() => {
  const area = document.getElementById('contentArea');
  area.scrollTop = area.scrollHeight;
});
await page.waitForTimeout(300);

// Trigger 7 wheel events to cross the threshold
console.log('=== Triggering chapter navigation via wheel ===');
for (let i = 0; i < 7; i++) {
  await page.mouse.wheel(0, 50);
  await page.waitForTimeout(500);
}

// Navigate back and forth a few times via buttons
console.log('=== Button navigations ===');
for (let i = 0; i < 5; i++) {
  await page.click('#btnPrev');
  await page.waitForTimeout(300);
  await page.click('#btnNext');
  await page.waitForTimeout(300);
}

// Check errors again
const newErrors = logs.filter(l => l.type === 'error');
console.log(`Total errors: ${newErrors.length}`);

// Check if any forced reflow warnings
const newWarns = logs.filter(l => l.type === 'warning');
console.log(`Total warnings: ${newWarns.length}`);
newWarns.forEach(w => console.log(`  ${w.text}`));

// Check excessive paint areas
console.log('=== Checking for potential flicker causes ===');

// 1. Does renderChapter cause a flash?
await page.evaluate(() => {
  const area = document.getElementById('contentArea');

  // Simulate rapid chapter switching
  const prevContent = document.getElementById('chapterContent').textContent;
  console.log(`[CHECK] contentArea listeners: scroll=${typeof area.onscroll}, wheel handler: present`);
  console.log(`[CHECK] _scrollCount=${_scrollCount}, _wasAtBottom=${_wasAtBottom}, _wasAtTop=${_wasAtTop}`);

  // Check if documentElement has inline background style (from eval)
  const htmlStyle = document.documentElement.style.backgroundColor;
  console.log(`[CHECK] html inline bg: "${htmlStyle}"`);
});

// Check the visual appearance by taking screenshots during navigation
await page.screenshot({ path: 'before-nav.png' });

await page.click('#btnNext');
await page.waitForTimeout(100); // Mid-transition
await page.screenshot({ path: 'during-nav.png' });

await page.waitForTimeout(500);
await page.screenshot({ path: 'after-nav.png' });

console.log('Screenshots saved: before-nav.png, during-nav.png, after-nav.png');

console.log('\n=== Analysis complete ===');

await page.waitForTimeout(3000);
await browser.close();
