import fs from "node:fs";

import { expect, test } from "@playwright/test";

test("themes can be customized and exported with the matching PDF", async ({ page }) => {
  const browserErrors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      browserErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => browserErrors.push(error.message));

  await page.goto("/");

  const status = page.locator("#status");
  const previews = page.locator("#preview img.page");
  await expect(status).toHaveText("Open a Markdown resume to begin.");
  await expect(previews).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download PDF" })).toHaveAttribute(
    "aria-disabled",
    "true",
  );

  await page.locator("#resume-file").setInputFiles("examples/resume.md");
  await expect(status).toContainText("Rendered 1 page(s)", { timeout: 120_000 });
  await expect(previews).toHaveCount(1);
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

  const firstPreviewUrl = await previews.first().getAttribute("src");
  await page.getByLabel("Theme", { exact: true }).selectOption("modern");
  await expect(status).toContainText("Rendered", { timeout: 120_000 });
  await expect
    .poll(() => previews.first().getAttribute("src"))
    .not.toBe(firstPreviewUrl);

  const bodySize = page.locator('#theme-controls input[type="range"]').first();
  await bodySize.evaluate((input: HTMLInputElement) => {
    input.value = "12";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(status).toContainText("Rendered", { timeout: 120_000 });

  await page.getByLabel("Edit Typst source").check();
  const source = page.getByLabel("Theme Typst source");
  await source.fill(`${await source.inputValue()}\n#missing-theme-function()`);
  await expect(status).toContainText("Theme error", { timeout: 120_000 });
  await expect(page.getByText("This preview is from the last theme that compiled.")).toBeVisible();
  await expect(previews).not.toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download PDF" })).toHaveAttribute(
    "aria-disabled",
    "true",
  );

  await page.getByRole("button", { name: "Reset" }).click();
  await expect(status).toContainText("Rendered", { timeout: 120_000 });
  await expect(page.getByRole("link", { name: "Download PDF" })).toHaveAttribute(
    "aria-disabled",
    "false",
  );

  const workers = page.workers();
  expect(workers).toHaveLength(1);
  expect(workers[0].url()).toContain("resumark-worker_loader.js");

  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download PDF" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("resume.pdf");

  const downloadedPath = await download.path();
  expect(downloadedPath).not.toBeNull();
  const pdfHeader = fs.readFileSync(downloadedPath!).subarray(0, 5).toString();
  expect(pdfHeader).toBe("%PDF-");

  const themeDownloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download theme" }).click();
  const themeDownload = await themeDownloadPromise;
  expect(themeDownload.suggestedFilename()).toBe("modern.typ");

  await page.screenshot({ path: "target/theme-workbench.png", fullPage: true });
  expect(browserErrors).toEqual([]);

  console.log(await status.textContent());
});
