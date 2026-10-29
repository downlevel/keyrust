import { statSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { openApp, unlock } from "./helpers";

// README screenshots of the demo (`pnpm screenshots`, never in CI). Demo data
// only; no password is shown in clear.

const OUT = resolve(import.meta.dirname, "../../../../docs/screenshots");
const MAX_BYTES = 400 * 1024;
// 2× unless the PNGs get too large without an optimiser: then 1.5×.
const SCALE = Number(process.env.KEYRUST_SCREENSHOT_SCALE ?? 2);

test.use({ deviceScaleFactor: SCALE, viewport: { width: 1100, height: 720 } });

async function shoot(page: Page, name: string, mask: string[] = []) {
	await page.evaluate(() => document.fonts.ready);
	const path = resolve(OUT, `${name}.png`);
	await page.screenshot({
		path,
		animations: "disabled",
		caret: "hide",
		mask: mask.map((s) => page.locator(s)),
		maskColor: "#f4f4f5",
	});
	expect(statSync(path).size, `${name}.png size`).toBeLessThan(MAX_BYTES);
}

test("unlock", async ({ page }) => {
	await openApp(page);
	await shoot(page, "unlock");
});

test("vault", async ({ page }) => {
	await unlock(page);
	await page.getByRole("option", { name: /GitHub/ }).click();
	await expect(page.getByTestId("password-value")).toHaveText("••••••••••••");
	await page.mouse.move(0, 0);
	await shoot(page, "vault");
});

test("editor with the generator", async ({ page }) => {
	await unlock(page);
	await page.getByRole("option", { name: /GitHub/ }).click();
	await page.getByRole("button", { name: "Edit" }).click();
	await page.getByRole("button", { name: "Generate", exact: true }).click();
	await expect(page.getByTestId("generated-password")).not.toBeEmpty();
	// The generated password is a real random value: masked.
	await shoot(page, "editor-generator", ['[data-testid="generated-password"]']);
});

test("settings", async ({ page }) => {
	await unlock(page);
	await page.keyboard.press("Control+,");
	await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
	await shoot(page, "settings");
});
