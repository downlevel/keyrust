import { expect, test, type Page } from "@playwright/test";
import { unlock } from "./helpers";

/** Longest animation and transition (seconds) on any element, and smooth scrolling. */
async function motion(page: Page) {
	return page.evaluate(() => {
		const seconds = (list: string) =>
			Math.max(
				...list.split(",").map((t) => {
					const v = parseFloat(t);
					return t.trim().endsWith("ms") ? v / 1000 : v;
				}),
			);
		let animation = 0;
		let transition = 0;
		let smooth: string[] = [];
		for (const el of [document.documentElement, ...document.querySelectorAll("body *")]) {
			for (const pseudo of [null, "::before", "::after"]) {
				const s = getComputedStyle(el, pseudo);
				if (s.animationName !== "none")
					animation = Math.max(animation, seconds(s.animationDuration));
				transition = Math.max(transition, seconds(s.transitionDuration));
				if (pseudo === null && s.scrollBehavior === "smooth")
					smooth = [...smooth, el.tagName];
			}
		}
		return { animation, transition, smooth };
	});
}

/** Opens each animated overlay in turn and returns the worst motion seen. */
async function overlays(page: Page) {
	const seen: Awaited<ReturnType<typeof motion>>[] = [];
	await unlock(page);
	await page.keyboard.press("Control+,");
	await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
	seen.push(await motion(page));
	await page.keyboard.press("Escape");
	await expect(page.getByRole("dialog", { name: "Settings" })).toBeHidden();
	await page.keyboard.press("Control+k");
	await expect(page.getByRole("dialog", { name: "Command palette" })).toBeVisible();
	seen.push(await motion(page));
	await page.keyboard.press("Escape");
	await expect(page.getByRole("dialog", { name: "Command palette" })).toBeHidden();
	await page.getByRole("option", { name: /GitHub/ }).click();
	await page.getByRole("button", { name: "Copy username" }).click();
	await expect(page.getByTestId("toast")).toBeVisible();
	seen.push(await motion(page));
	await page.getByRole("button", { name: "Edit" }).click();
	await page.getByRole("button", { name: "Generate", exact: true }).click();
	await expect(page.getByRole("dialog", { name: "Password generator" })).toBeVisible();
	seen.push(await motion(page));
	return {
		animation: Math.max(...seen.map((s) => s.animation)),
		transition: Math.max(...seen.map((s) => s.transition)),
		smooth: seen.flatMap((s) => s.smooth),
	};
}

test.describe("reduced motion", () => {
	test.use({ contextOptions: { reducedMotion: "reduce" } });

	test("no animation or transition longer than 0.01 s, no smooth scrolling", async ({ page }) => {
		const worst = await overlays(page);
		expect(worst.animation).toBeLessThanOrEqual(0.01);
		expect(worst.transition).toBeLessThanOrEqual(0.01);
		expect(worst.smooth).toEqual([]);
	});
});

test("without the preference the animations remain", async ({ page }) => {
	const worst = await overlays(page);
	expect(worst.animation).toBeGreaterThan(0.01);
	expect(worst.transition).toBeGreaterThan(0.01);
});
