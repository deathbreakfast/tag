import AxeBuilder from "@axe-core/playwright";
import { test, expect, waitForHydrated } from "./fixtures";
import type { Locator, Page } from "@playwright/test";

/**
 * TagCatalogPicker on Orbital's ChipSelector, mounted with linked chips at
 * `/dev/tag-picker`. Each check runs on desktop light, desktop dark and phone.
 */
const LOOKS = [
  { name: "desktop-light", viewport: { width: 1280, height: 800 }, colorScheme: "light" },
  { name: "desktop-dark", viewport: { width: 1280, height: 800 }, colorScheme: "dark" },
  { name: "phone", viewport: { width: 390, height: 844 }, colorScheme: "light" },
] as const;

type SeedTag = { id: string; name: string };

const demo = (page: Page) => page.getByTestId("tag-picker-demo");
const picker = (page: Page) => demo(page).getByTestId("tag-catalog-picker");
const control = (page: Page) => picker(page).locator(".orbital-tag-picker-control");
const chips = (page: Page) => picker(page).locator(".orbital-tag");
const input = (page: Page) => picker(page).locator("input[role='combobox']");
const option = (page: Page, name: string): Locator =>
  page.locator(".orbital-tag-picker-option", { hasText: name });
const lastChange = (page: Page) => demo(page).getByTestId("tag-picker-demo-change");

/** Seed the owner session (plus `extra` owner tags), then open the demo with `selected` chips. */
async function openDemo(page: Page, selected: "fixture" | "extra" | "none", extra = 0) {
  const res = await page.request.post("/api/test/seed-data", {
    data: { auth: "owner", seed_catalog: true, extra_owner_tags: extra },
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  const body = (await res.json()) as {
    fixtures: { tag_id: string; tag_name: string };
    extra_tags: SeedTag[];
  };
  const fixture = { id: body.fixtures.tag_id, name: body.fixtures.tag_name };
  const seed =
    selected === "fixture" ? [fixture] : selected === "extra" ? body.extra_tags : [];
  const query = seed.length ? `?seed=${encodeURIComponent(JSON.stringify(seed))}` : "";
  await page.goto(`/dev/tag-picker${query}`, { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
  await expect(picker(page)).toBeVisible({ timeout: 60_000 });
  return { fixture, extra: body.extra_tags };
}

for (const look of LOOKS) {
  test.describe(`pw-tag-picker [${look.name}]`, () => {
    test.use({ viewport: look.viewport, colorScheme: look.colorScheme });

    test(`pw-tag-picker-pick-happy [${look.name}]`, async ({ page }) => {
      const { fixture } = await openDemo(page, "none");
      await expect(chips(page)).toHaveCount(0);
      await input(page).fill(fixture.name);
      await option(page, fixture.name).click();
      await expect(control(page).locator(".orbital-tag")).toHaveCount(1);
      await expect(chips(page)).toHaveText(fixture.name);
      await expect(lastChange(page)).toContainText(fixture.id);
    });

    test(`pw-tag-picker-remove-last-happy [${look.name}]`, async ({ page }) => {
      const { fixture } = await openDemo(page, "fixture");
      await expect(chips(page)).toHaveCount(1);
      await picker(page).getByRole("button", { name: `Remove ${fixture.name}` }).click();
      await expect(chips(page)).toHaveCount(0);
      await expect(input(page)).toHaveAttribute("placeholder", "No tags. Search or create one");
      // The host gets an empty list, not a missing one.
      await expect(lastChange(page)).toHaveText("[]");
    });

    test(`pw-tag-picker-many-wrap-happy [${look.name}]`, async ({ page }) => {
      const { extra } = await openDemo(page, "extra", 9);
      await expect(chips(page)).toHaveCount(9);
      await expect(chips(page).last()).toContainText(extra[8].name);
      const box = await control(page).boundingBox();
      expect(box).not.toBeNull();
      const first = await chips(page).first().boundingBox();
      const last = await chips(page).last().boundingBox();
      // More than one row of chips.
      expect(last!.y).toBeGreaterThan(first!.y + first!.height / 2);
      const overflow = await control(page).evaluate((el) => el.scrollWidth - el.clientWidth);
      expect(overflow).toBeLessThanOrEqual(1);
      for (const chip of await chips(page).all()) {
        const c = await chip.boundingBox();
        expect(c!.x).toBeGreaterThanOrEqual(box!.x - 1);
        expect(c!.x + c!.width).toBeLessThanOrEqual(box!.x + box!.width + 1);
      }
    });

    test(`pw-tag-picker-link-happy [${look.name}]`, async ({ page }) => {
      const { fixture } = await openDemo(page, "fixture");
      const link = picker(page).locator(`a[href='/tag/${fixture.id}']`);
      await expect(link).toHaveText(fixture.name);
      await link.click();
      await expect(page).toHaveURL(new RegExp(`/tag/${fixture.id}$`), { timeout: 60_000 });
      await expect(page.getByTestId("tag-detail-page")).toBeVisible({ timeout: 60_000 });
    });

    test(`pw-tag-picker-dismiss-sad [${look.name}]`, async ({ page }) => {
      const { fixture } = await openDemo(page, "fixture");
      const before = page.url();
      await picker(page).getByRole("button", { name: `Remove ${fixture.name}` }).click();
      await expect(chips(page)).toHaveCount(0);
      expect(page.url()).toBe(before);
      await expect(demo(page)).toBeVisible();
    });

    test(`pw-tag-picker-create-happy [${look.name}]`, async ({ page }) => {
      await openDemo(page, "none");
      const name = `Picker New ${Date.now()}`;
      await input(page).fill(name);
      await option(page, `${name} (new)`).click();
      const dialog = page.getByTestId("tag-create-dialog");
      await expect(dialog).toBeVisible({ timeout: 30_000 });
      await expect(dialog.getByRole("textbox", { name: /^Name/ })).toHaveValue(name);
      // The placeholder key never reaches the host.
      await expect(lastChange(page)).not.toContainText("__create");
      await page.getByTestId("tag-create-dialog-submit").click();
      await expect(chips(page)).toHaveCount(1, { timeout: 30_000 });
      await expect(chips(page)).toHaveText(name);
    });

    test(`pw-tag-picker-axe-happy [${look.name}]`, async ({ page }) => {
      await openDemo(page, "fixture");
      await input(page).click();
      const results = await new AxeBuilder({ page })
        .include('[data-testid="tag-picker-demo"]')
        .analyze();
      expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
    });
  });
}
