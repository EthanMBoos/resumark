import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/browser",
  outputDir: "target/playwright-results",
  reporter: "line",
  timeout: 180_000,
  workers: 1,
  use: {
    baseURL: "http://127.0.0.1:8080",
    browserName: "chromium",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "python3 -m http.server 8080 --directory target/web-dist",
    url: "http://127.0.0.1:8080",
    reuseExistingServer: false,
    timeout: 10_000,
  },
});
