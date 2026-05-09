import { expect, type Page, test } from "@playwright/test";

const appPath = process.env.PANEL_UI_APP_PATH ?? "/";
const panelApiRoot = "/panel/api";
const assetRoot =
  appPath === "/" ? "/assets" : `${appPath.replace(/\/$/, "")}/assets`;
const prompts = [
  "Explain xavier memory and show the answer as a structured UI.",
  "Summarize the current agent workflow as cards with supporting details.",
];

async function enterPanel(page: Page) {
  await page.goto(appPath);

  await expect(
    page.getByText("OpenUI cockpit for the internal agent"),
  ).toBeVisible();

  await page.getByPlaceholder("XAVIER_TOKEN").fill("dev-token");
  await page.getByRole("button", { name: "Enter panel" }).click();

  await expect(page.getByRole("button", { name: "New thread" })).toBeVisible();
}

test.describe("Xavier generative panel", () => {
  test("keeps the shell public while protecting panel APIs and assets", async ({
    page,
    request,
  }) => {
    const shellResponse = await request.get(appPath);
    expect(shellResponse.status()).toBe(200);

    const assetResponse = await request.get(`${assetRoot}/index.js`);
    expect(assetResponse.status()).toBe(200);
    expect(assetResponse.headers()["content-type"]).toContain("javascript");

    const missingAssetResponse = await request.get(`${assetRoot}/missing.js`);
    expect(missingAssetResponse.status()).toBe(404);

    const unauthorizedThreadsResponse = await request.get(
      `${panelApiRoot}/threads`,
    );
    expect(unauthorizedThreadsResponse.status()).toBe(401);

    const authorizedThreadsResponse = await request.get(
      `${panelApiRoot}/threads`,
      {
        headers: { "X-Xavier-Token": "dev-token" },
      },
    );
    expect(authorizedThreadsResponse.status()).toBe(200);
    expect(await authorizedThreadsResponse.json()).toEqual(expect.any(Array));

    await enterPanel(page);
  });

  test("creates threads, preserves empty state, and renders structured assistant output", async ({
    page,
    request,
  }) => {
    await enterPanel(page);

    await page.getByRole("button", { name: "New thread" }).click();
    await expect(page.locator(".topbar h1")).toHaveText("New Thread");
    await expect(page.locator(".message-card")).toHaveCount(0);

    const composer = page.getByPlaceholder(
      "Ask Xavier for memory, code, or a structured answer...",
    );

    for (const prompt of prompts) {
      await composer.fill(prompt);
      await page.getByRole("button", { name: "Send" }).click();

      await expect(page.locator(".loading-block")).toBeVisible();
      await expect(page.locator(".loading-block")).toBeHidden({
        timeout: 45_000,
      });

      const renderSurface = page.locator(".render-surface").last();
      await expect(renderSurface).toBeVisible();
      await expect(
        renderSurface.getByText("OpenUI Render Surface"),
      ).toBeVisible();

      const assistantCards = page.locator(".assistant-card");
      await expect
        .poll(async () => assistantCards.count(), { timeout: 15_000 })
        .toBeGreaterThan(0);
      await expect(assistantCards.last().locator(".plain-text")).not.toHaveText(
        /^$/,
      );
    }

    await expect(page.locator(".topbar h1")).not.toHaveText("New Thread");

    const threadsResponse = await request.get(`${panelApiRoot}/threads`, {
      headers: { "X-Xavier-Token": "dev-token" },
    });
    const threads = (await threadsResponse.json()) as Array<{
      id: string;
      title: string;
    }>;
    const activeThread = threads[0];

    expect(activeThread).toBeDefined();

    const detailResponse = await request.get(
      `${panelApiRoot}/threads/${activeThread?.id}`,
      {
        headers: { "X-Xavier-Token": "dev-token" },
      },
    );
    expect(detailResponse.status()).toBe(200);

    const detail = (await detailResponse.json()) as {
      thread: { title: string };
      messages: Array<{
        role: string;
        plain_text: string;
        openui_lang?: string | null;
      }>;
    };

    expect(detail.thread.title).not.toBe("New Thread");
    expect(detail.messages).toHaveLength(prompts.length * 2);
    expect(detail.messages.at(-1)?.role).toBe("assistant");
    expect(detail.messages.at(-1)?.plain_text).not.toHaveLength(0);
    expect(detail.messages.at(-1)?.openui_lang).toEqual(expect.any(String));
  });
});
