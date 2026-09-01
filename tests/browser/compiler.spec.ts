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
  await page.getByLabel("Theme", { exact: true }).selectOption("pirate");
  await expect(status).toContainText("Rendered 1 page(s)", { timeout: 120_000 });
  await expect(page.getByRole("group", { name: "Text size" })).toContainText("9");
  await expect(page.getByRole("group", { name: "Name size" })).toContainText("16");

  await page.getByLabel("Theme", { exact: true }).selectOption("modern");
  await expect(status).toContainText("Rendered", { timeout: 120_000 });
  await expect
    .poll(() => previews.first().getAttribute("src"))
    .not.toBe(firstPreviewUrl);

  await expect(page.getByText("Customize", { exact: true })).toBeVisible();
  await expect(page.getByRole("group", { name: "Text size" })).toContainText("10.5");
  await page.getByRole("button", { name: "Increase Text size" }).click();
  await page.getByRole("slider", { name: "Spacing" }).fill("73");
  await page.getByRole("slider", { name: "Page margins" }).fill("67");
  await expect(page.getByRole("slider", { name: "Spacing" })).toHaveValue("73");
  await expect(page.getByRole("slider", { name: "Page margins" })).toHaveValue("67");
  await expect(status).toContainText("Rendered", { timeout: 120_000 });

  await page.locator("#theme-file").setInputFiles({
    name: "broken.typ",
    mimeType: "text/plain",
    buffer: Buffer.from("#missing-theme-manifest()"),
  });
  await expect(status).toContainText("Theme error", { timeout: 120_000 });
  await expect(page.getByText("This preview is from the last theme that compiled.")).toBeVisible();
  await expect(previews).not.toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download PDF" })).toHaveAttribute(
    "aria-disabled",
    "true",
  );

  await page.getByLabel("Theme", { exact: true }).selectOption("modern");
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

  await page.getByLabel("Theme", { exact: true }).selectOption("pirate");
  await expect(status).toContainText("Rendered 1 page(s)", { timeout: 120_000 });

  await page.getByText("Theme files", { exact: true }).click();
  const themeDownloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download theme" }).click();
  const themeDownload = await themeDownloadPromise;
  expect(themeDownload.suggestedFilename()).toBe("pirate.typ");

  await page.screenshot({ path: "target/theme-workbench.png", fullPage: true });
  expect(browserErrors).toEqual([]);

  console.log(await status.textContent());
});
