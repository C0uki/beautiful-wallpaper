// Renders every surface and saves a screenshot of each.
//
// The shell targets Windows, but its UI is a webview: served against the mock
// backend it renders identically anywhere Chromium runs. That makes this the
// practical way to review the surfaces — and to catch a layout or theming
// regression — without a Windows machine.
//
//   pnpm --filter @bw/shell screenshots
//
// Output lands in screenshots/ at the repository root.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { chromium } from "@playwright/test";

// Some environments ship a Chromium that does not match the build Playwright
// expects. Point at it explicitly rather than downloading a second copy.
const PRESET_CHROMIUM = process.env.BW_CHROMIUM ?? "/opt/pw-browsers/chromium";
const launchOptions = existsSync(PRESET_CHROMIUM)
  ? { executablePath: PRESET_CHROMIUM, args: ["--no-sandbox", "--use-gl=swiftshader"] }
  : { args: ["--no-sandbox"] };

const appDir = fileURLToPath(new URL("..", import.meta.url));
const outDir = path.resolve(appDir, "../../screenshots");
// Two of the shots are also written at 1x into docs/images/, small enough to
// commit so the READMEs have something to show.
const docsDir = path.resolve(appDir, "../../docs/images");
const DOC_SHOTS = new Set(["03-both", "01-desktop"]);
const PORT = 4173;
const HOST = "127.0.0.1";
const BASE = `http://${HOST}:${PORT}`;

/** Starts `vite preview` and resolves once it is answering. */
async function startPreview() {
  const server = spawn(
    "pnpm",
    [
      "exec",
      "vite",
      "preview",
      "--port",
      String(PORT),
      "--strictPort",
      // Bind exactly where the probe below looks. Vite's default is
      // `localhost`, which can resolve to ::1 only — the server then comes up
      // fine and every request to 127.0.0.1 is refused until the deadline.
      "--host",
      HOST,
    ],
    { cwd: appDir, stdio: ["ignore", "pipe", "pipe"] },
  );

  // Both streams must be drained: a full pipe buffer blocks the child mid-write,
  // and the output is the only evidence of why a start failed.
  let output = "";
  for (const stream of [server.stdout, server.stderr]) {
    stream.setEncoding("utf8");
    stream.on("data", (chunk) => {
      output += chunk;
    });
  }

  let exit = null;
  server.on("exit", (code, signal) => {
    exit = signal ? `signal ${signal}` : `exit code ${code}`;
  });

  // Generous, because a cold CI runner is much slower than a warm checkout and
  // the cost of waiting is only paid when something is actually wrong.
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${BASE}/index.html`);
      if (response.ok) return server;
    } catch {
      // Not up yet.
    }
    // A server that has already quit is never going to answer, and its output
    // says why — far better than spending the rest of the deadline on it.
    if (exit !== null) {
      throw new Error(`vite preview quit before serving (${exit})\n${output.trim()}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  server.kill();
  throw new Error(`vite preview did not answer on ${BASE} within 60s\n${output.trim()}`);
}

/** Waits for the surface to have painted its theme and its wallpaper. */
async function settle(page) {
  await page.waitForFunction(() => {
    const value = getComputedStyle(document.documentElement).getPropertyValue("--m3-primary");
    return value.trim().length > 0;
  });
  // The wallpaper transition and the widget entrance both finish well inside this.
  await page.waitForTimeout(1600);
}

const SHOTS = [
  { name: "01-desktop", url: "/index.html", setup: async (page) => selectView(page, "Desktop") },
  { name: "02-wallpaper-picker", url: "/index.html", setup: async (page) => selectView(page, "Picker") },
  { name: "03-both", url: "/index.html" },
  {
    name: "04-light-mode",
    url: "/index.html",
    setup: async (page) => {
      await selectView(page, "Desktop");
      await page.getByRole("button", { name: /^dark$/ }).click();
    },
  },
  {
    name: "05-japanese",
    url: "/index.html",
    setup: async (page) => {
      // Language first: switching it remounts the surface, which would reset the
      // view back to its default.
      await page.getByRole("tab", { name: "日本語" }).click();
      await page.waitForTimeout(600);
      await selectView(page, "Picker");
    },
  },
  {
    name: "06-widget-edit-mode",
    url: "/index.html",
    setup: async (page) => {
      await selectView(page, "Desktop");
      await page.getByRole("button", { name: "Edit widgets" }).click();
    },
  },
  {
    name: "07-bar-styles",
    url: "/index.html",
    setup: async (page) => {
      await selectView(page, "Desktop");
      await page.getByLabel("Bar style").selectOption("islands");
    },
  },
  {
    name: "08-bar-at-the-bottom",
    url: "/index.html",
    setup: async (page) => {
      await selectView(page, "Desktop");
      await page.getByRole("button", { name: /^top$/ }).click();
    },
  },
  { name: "09-background-surface", url: "/background.html" },
  // The bar window is only as tall as the bar itself, so previewing it in a
  // full-height viewport would be misleading.
  { name: "10-bar-surface", url: "/bar.html", viewport: { width: 1600, height: 48 } },
  { name: "11-wallpaper-selector-surface", url: "/wallpaperSelector.html" },
  // Both windows are sized to their content on Windows, so the viewports here
  // match the fraction of the screen each surface declares.
  { name: "12-osd", url: "/osd.html", viewport: { width: 420, height: 110 } },
  {
    name: "13-notification-toasts",
    url: "/notifications.html",
    viewport: { width: 460, height: 460 },
  },
];

async function selectView(page, label) {
  await page.getByRole("tab", { name: label }).click();
}

/** A small JPEG of the same view, sized to be committed alongside the READMEs. */
async function captureForDocs(browser, shot) {
  const context = await browser.newContext({
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
    locale: "en-US",
  });
  const page = await context.newPage();
  await page.goto(`${BASE}${shot.url}`, { waitUntil: "load" });
  await settle(page);
  if (shot.setup) {
    await shot.setup(page);
    await page.waitForTimeout(1400);
  }
  await page.screenshot({
    path: path.join(docsDir, `${shot.name}.jpg`),
    type: "jpeg",
    quality: 82,
  });
  await context.close();
}

async function main() {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });
  await mkdir(docsDir, { recursive: true });

  const server = await startPreview();
  const browser = await chromium.launch(launchOptions);
  const failures = [];

  try {
    for (const shot of SHOTS) {
      const context = await browser.newContext({
        viewport: shot.viewport ?? { width: 1600, height: 900 },
        deviceScaleFactor: 2,
        locale: "en-US",
      });
      const page = await context.newPage();

      const consoleErrors = [];
      page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
      });
      page.on("pageerror", (error) => consoleErrors.push(String(error)));

      await page.goto(`${BASE}${shot.url}`, { waitUntil: "load" });
      await settle(page);
      if (shot.setup) {
        await shot.setup(page);
        await page.waitForTimeout(1400);
      }

      const file = path.join(outDir, `${shot.name}.png`);
      await page.screenshot({ path: file });

      if (consoleErrors.length > 0) {
        failures.push(`${shot.name}: ${consoleErrors.join(" | ")}`);
      }
      console.log(`${consoleErrors.length ? "✗" : "✓"} ${shot.name}`);
      await context.close();

      if (DOC_SHOTS.has(shot.name)) {
        await captureForDocs(browser, shot);
      }
    }
  } finally {
    await browser.close();
    server.kill();
  }

  if (failures.length > 0) {
    console.error("\nConsole errors while rendering:");
    for (const failure of failures) console.error(`  ${failure}`);
    process.exitCode = 1;
  } else {
    console.log(`\n${SHOTS.length} screenshots in ${outDir}`);
  }
}

await main();
