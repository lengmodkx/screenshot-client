import { chromium } from '@playwright/test';

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 420, height: 800 }
  });

  await page.goto('file://' + process.cwd() + '/preview.html');
  await page.waitForLoadState('domcontentloaded');

  await page.screenshot({ path: 'screenshot-preview.png', fullPage: true });

  console.log('Screenshot saved to screenshot-preview.png');

  await browser.close();
})();
