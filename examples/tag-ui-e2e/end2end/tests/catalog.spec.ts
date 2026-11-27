import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-tag-list", () => {
  test("pw-tag-list-load-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "owner");
    await page.goto("/tag", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-list-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(fixtures.tag_name, { exact: true })).toBeVisible({
      timeout: 60_000,
    });
    // Updated renders through Orbital Timestamp: the date in the cell, the
    // full instant (UTC included) in the hover card.
    const row = page.getByRole("row").filter({ hasText: fixtures.tag_name });
    const updated = row.getByTestId("tag-row-updated").locator("time");
    await expect(updated).toHaveAttribute("datetime", /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/);
    await updated.hover();
    await expect(page.getByTestId("timestamp-card-utc").first()).toBeVisible();
  });

  test("pw-tag-search-miss-sad", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/tag", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-list-page")).toBeVisible({
      timeout: 60_000,
    });
    await page.getByPlaceholder(/Search tags/i).fill("zz-no-such-tag-xyz");
    await expect(page.getByTestId("tag-list-filtered-empty")).toBeVisible({
      timeout: 30_000,
    });
  });
});

test.describe("pw-tag-create", () => {
  test("pw-tag-create-happy", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/tag/create", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-create-page")).toBeVisible({
      timeout: 60_000,
    });
    const name = `E2E-Create-${Date.now()}`;
    await page.getByRole("textbox", { name: /^Name/ }).fill(name);
    await page.getByRole("textbox", { name: /^Taxonomy/ }).fill("spend");
    await page.getByTestId("tag-create-submit").click();
    await expect(page).toHaveURL(/\/tag\/[^/]+$/, { timeout: 60_000 });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-detail-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(name, { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByRole("textbox", { name: /^Name$/ })).toHaveValue(name);
  });
});

test.describe("pw-tag-detail", () => {
  test("pw-tag-update-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "owner", true);
    await page.goto(`/tag/${fixtures.tag_id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-detail-page")).toBeVisible({
      timeout: 60_000,
    });
    const updated = `${fixtures.tag_name}-edited`;
    await page.getByRole("textbox", { name: /^Name$/ }).fill(updated);
    await page.getByTestId("tag-detail-save").click();
    await expect(page.getByRole("textbox", { name: /^Name$/ })).toHaveValue(updated, {
      timeout: 60_000,
    });
    await expect(page.getByText(updated, { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
  });

  test("pw-tag-delete-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "owner", true);
    await page.goto(`/tag/${fixtures.tag_id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-detail-page")).toBeVisible({
      timeout: 60_000,
    });
    await page.getByTestId("tag-detail-delete").click();
    await expect(page).toHaveURL(/\/tag\/?$/, { timeout: 60_000 });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-list-page")).toBeVisible({
      timeout: 60_000,
    });
    // Soft-delete: row may remain listed until deletion DAG finalize. Confirm
    // the detail surface no longer loads the live editor for this id.
    await page.goto(`/tag/${fixtures.tag_id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-detail-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("tag-detail-save")).toHaveCount(0);
    await expect(
      page.getByText(/Tag not found|Pending deletion|Access denied/i).first(),
    ).toBeVisible({ timeout: 30_000 });
  });

  test("pw-tag-not-found-sad", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/tag/missing-id-does-not-exist", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("tag-detail-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(/Tag not found/i)).toBeVisible({
      timeout: 30_000,
    });
  });
});
