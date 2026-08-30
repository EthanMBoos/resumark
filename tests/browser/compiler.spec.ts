import fs from "node:fs";

import { expect, test } from "@playwright/test";

test("the worker renders previews and a PDF", async ({ page }) => {
  const browserErrors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      browserErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => browserErrors.push(error.message));

  await page.goto("/");

  const status = page.locator("#status");
  await expect(status).toContainText("Rendered 2 page(s)", { timeout: 120_000 });
  const previews = page.locator("#preview img.page");
  await expect(previews).toHaveCount(2);
  await expect(previews.first()).toBeVisible();
  await expect(previews.last()).toBeVisible();
  await expect
    .poll(() =>
      previews.evaluateAll((images) =>
        images.every((image) => {
          const preview = image as HTMLImageElement;
          return preview.complete && preview.naturalWidth > 0;
        }),
      ),
    )
    .toBe(true);

  const workers = page.workers();
  expect(workers).toHaveLength(1);
  expect(workers[0].url()).toContain("resumark-worker_loader.js");

  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download PDF" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("jane-doe-resume.pdf");

  const downloadedPath = await download.path();
  expect(downloadedPath).not.toBeNull();
  const pdfHeader = fs.readFileSync(downloadedPath!).subarray(0, 5).toString();
  expect(pdfHeader).toBe("%PDF-");

  await page.screenshot({ path: "target/stage3-browser.png", fullPage: true });
  expect(browserErrors).toEqual([]);

  console.log(await status.textContent());
});
