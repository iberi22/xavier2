# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: generative-ui.spec.ts >> Xavier generative panel >> creates threads, preserves empty state, and renders structured assistant output
- Location: tests/generative-ui.spec.ts:57:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.loading-block')
Expected: visible
Timeout: 15000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 15000ms
  - waiting for locator('.loading-block')

```

# Page snapshot

```yaml
- generic [ref=e3]:
  - complementary [ref=e4]:
    - generic [ref=e5]:
      - generic [ref=e6]:
        - paragraph [ref=e7]: Xavier
        - heading "Render Agent Console" [level=2] [ref=e8]
      - button "New thread" [ref=e9] [cursor=pointer]
    - generic [ref=e10]:
      - generic [ref=e11]:
        - generic [ref=e12]: ok
        - generic [ref=e13]: Live backend
      - paragraph [ref=e14]: Reasoning agent + render agent are split and persisted per thread.
    - generic [ref=e15]:
      - 'button "Thread Explain xavier memory and show the answer as a s Structured Xavier response for: Explain xavier memory and show the answer as a structured UI. 2 messages" [ref=e16] [cursor=pointer]':
        - generic [ref=e17]: Thread
        - strong [ref=e18]: Explain xavier memory and show the answer as a s
        - generic [ref=e19]: "Structured Xavier response for: Explain xavier memory and show the answer as a structured UI."
        - generic [ref=e20]: 2 messages
      - 'button "Thread Explain xavier memory and show the answer as a s Structured Xavier response for: Explain xavier memory and show the answer as a structured UI. 2 messages" [ref=e21] [cursor=pointer]':
        - generic [ref=e22]: Thread
        - strong [ref=e23]: Explain xavier memory and show the answer as a s
        - generic [ref=e24]: "Structured Xavier response for: Explain xavier memory and show the answer as a structured UI."
        - generic [ref=e25]: 2 messages
  - main [ref=e26]:
    - generic [ref=e27]:
      - generic [ref=e28]:
        - paragraph [ref=e29]: Protected endpoint
        - heading "Explain xavier memory and show the answer as a s" [level=1] [ref=e30]
      - generic [ref=e31]:
        - generic [ref=e32]:
          - text: Threads
          - strong [ref=e33]: "2"
        - generic [ref=e34]:
          - text: Messages
          - strong [ref=e35]: "2"
    - generic [ref=e36]:
      - article [ref=e37]:
        - generic [ref=e38]:
          - strong [ref=e39]: Operator
          - generic [ref=e40]: 2:26:22 AM
        - generic [ref=e41]: Explain xavier memory and show the answer as a structured UI.
      - article [ref=e42]:
        - generic [ref=e43]:
          - strong [ref=e44]: Xavier UI Agent
          - generic [ref=e45]: 2:26:22 AM
        - generic [ref=e46]:
          - generic [ref=e47]:
            - text: Confidence
            - strong [ref=e48]: n/a
          - generic [ref=e49]:
            - text: Documents
            - strong [ref=e50]: "0"
          - generic [ref=e51]:
            - text: Evidence
            - strong [ref=e52]: "0"
          - generic [ref=e53]:
            - text: Latency
            - strong [ref=e54]: 0 ms
        - generic [ref=e55]:
          - generic [ref=e56]:
            - heading "Render rules" [level=3] [ref=e57]
            - generic [ref=e58]:
              - generic [ref=e59]: deterministic
              - generic [ref=e60]: ci-safe
          - generic [ref=e61]:
            - heading "Components" [level=3] [ref=e62]
            - generic [ref=e63]:
              - generic [ref=e64]: SectionBlock
              - generic [ref=e65]: InfoCard
        - generic [ref=e67]:
          - generic [ref=e68]: OpenUI Render Surface
          - generic [ref=e69]: Structured output
        - generic [ref=e70]: "Structured Xavier response for: Explain xavier memory and show the answer as a structured UI."
    - generic [ref=e71]:
      - textbox "Ask Xavier for memory, code, or a structured answer..." [ref=e72]
      - button "Send" [active] [ref=e73] [cursor=pointer]
```

# Test source

```ts
  1   | import { expect, type Page, test } from "@playwright/test";
  2   |
  3   | const appPath = process.env.PANEL_UI_APP_PATH ?? "/";
  4   | const panelApiRoot = "/panel/api";
  5   | const assetRoot =
  6   |   appPath === "/" ? "/assets" : `${appPath.replace(/\/$/, "")}/assets`;
  7   | const prompts = [
  8   |   "Explain xavier memory and show the answer as a structured UI.",
  9   |   "Summarize the current agent workflow as cards with supporting details.",
  10  | ];
  11  |
  12  | async function enterPanel(page: Page) {
  13  |   await page.goto(appPath);
  14  |
  15  |   await expect(
  16  |     page.getByText("OpenUI cockpit for the internal agent"),
  17  |   ).toBeVisible();
  18  |
  19  |   await page.getByPlaceholder("XAVIER_TOKEN").fill("dev-token");
  20  |   await page.getByRole("button", { name: "Enter panel" }).click();
  21  |
  22  |   await expect(page.getByRole("button", { name: "New thread" })).toBeVisible();
  23  | }
  24  |
  25  | test.describe("Xavier generative panel", () => {
  26  |   test("keeps the shell public while protecting panel APIs and assets", async ({
  27  |     page,
  28  |     request,
  29  |   }) => {
  30  |     const shellResponse = await request.get(appPath);
  31  |     expect(shellResponse.status()).toBe(200);
  32  |
  33  |     const assetResponse = await request.get(`${assetRoot}/index.js`);
  34  |     expect(assetResponse.status()).toBe(200);
  35  |     expect(assetResponse.headers()["content-type"]).toContain("javascript");
  36  |
  37  |     const missingAssetResponse = await request.get(`${assetRoot}/missing.js`);
  38  |     expect(missingAssetResponse.status()).toBe(404);
  39  |
  40  |     const unauthorizedThreadsResponse = await request.get(
  41  |       `${panelApiRoot}/threads`,
  42  |     );
  43  |     expect(unauthorizedThreadsResponse.status()).toBe(401);
  44  |
  45  |     const authorizedThreadsResponse = await request.get(
  46  |       `${panelApiRoot}/threads`,
  47  |       {
  48  |         headers: { "X-Xavier-Token": "dev-token" },
  49  |       },
  50  |     );
  51  |     expect(authorizedThreadsResponse.status()).toBe(200);
  52  |     expect(await authorizedThreadsResponse.json()).toEqual(expect.any(Array));
  53  |
  54  |     await enterPanel(page);
  55  |   });
  56  |
  57  |   test("creates threads, preserves empty state, and renders structured assistant output", async ({
  58  |     page,
  59  |     request,
  60  |   }) => {
  61  |     await enterPanel(page);
  62  |
  63  |     await page.getByRole("button", { name: "New thread" }).click();
  64  |     await expect(page.locator(".topbar h1")).toHaveText("New Thread");
  65  |     await expect(page.locator(".message-card")).toHaveCount(0);
  66  |
  67  |     const composer = page.getByPlaceholder(
  68  |       "Ask Xavier for memory, code, or a structured answer...",
  69  |     );
  70  |
  71  |     for (const prompt of prompts) {
  72  |       await composer.fill(prompt);
  73  |       await page.getByRole("button", { name: "Send" }).click();
  74  |
> 75  |       await expect(page.locator(".loading-block")).toBeVisible();
      |                                                    ^ Error: expect(locator).toBeVisible() failed
  76  |       await expect(page.locator(".loading-block")).toBeHidden({
  77  |         timeout: 45_000,
  78  |       });
  79  |
  80  |       const renderSurface = page.locator(".render-surface").last();
  81  |       await expect(renderSurface).toBeVisible();
  82  |       await expect(
  83  |         renderSurface.getByText("OpenUI Render Surface"),
  84  |       ).toBeVisible();
  85  |
  86  |       const assistantCards = page.locator(".assistant-card");
  87  |       await expect
  88  |         .poll(async () => assistantCards.count(), { timeout: 15_000 })
  89  |         .toBeGreaterThan(0);
  90  |       await expect(assistantCards.last().locator(".plain-text")).not.toHaveText(
  91  |         /^$/,
  92  |       );
  93  |     }
  94  |
  95  |     await expect(page.locator(".topbar h1")).not.toHaveText("New Thread");
  96  |
  97  |     const threadsResponse = await request.get(`${panelApiRoot}/threads`, {
  98  |       headers: { "X-Xavier-Token": "dev-token" },
  99  |     });
  100 |     const threads = (await threadsResponse.json()) as Array<{
  101 |       id: string;
  102 |       title: string;
  103 |     }>;
  104 |     const activeThread = threads[0];
  105 |
  106 |     expect(activeThread).toBeDefined();
  107 |
  108 |     const detailResponse = await request.get(
  109 |       `${panelApiRoot}/threads/${activeThread?.id}`,
  110 |       {
  111 |         headers: { "X-Xavier-Token": "dev-token" },
  112 |       },
  113 |     );
  114 |     expect(detailResponse.status()).toBe(200);
  115 |
  116 |     const detail = (await detailResponse.json()) as {
  117 |       thread: { title: string };
  118 |       messages: Array<{
  119 |         role: string;
  120 |         plain_text: string;
  121 |         openui_lang?: string | null;
  122 |       }>;
  123 |     };
  124 |
  125 |     expect(detail.thread.title).not.toBe("New Thread");
  126 |     expect(detail.messages).toHaveLength(prompts.length * 2);
  127 |     expect(detail.messages.at(-1)?.role).toBe("assistant");
  128 |     expect(detail.messages.at(-1)?.plain_text).not.toHaveLength(0);
  129 |     expect(detail.messages.at(-1)?.openui_lang).toEqual(expect.any(String));
  130 |   });
  131 | });
  132 |
```