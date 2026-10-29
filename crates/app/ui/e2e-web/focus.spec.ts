import { expect, test, type Page } from "@playwright/test";
import { contrastRatio, parseColor } from "../src/lib/a11y/contrast";
import { openApp, palette, unlock } from "./helpers";

/**
 * Elements whose focus indicator is drawn by an ancestor: [element selector,
 * selector of the ancestor that carries the indicator]. Everything else
 * carries its own.
 */
const CARRIERS: [string, string][] = [
	// SecretInput: the ring is on the wrapper around the field and its toggle.
	[
		"#master-password, #create-password, #create-confirm, #editor-password, #new-master, #confirm-master",
		"div.focus-within\\:ring-2",
	],
	// Search field and the command palette field.
	['input[type="search"]', "div.focus-within\\:ring-2"],
	['[data-slot="command-input"]', '[data-slot="input-group"]'],
];

interface Style {
	outlineStyle: string;
	outlineWidth: string;
	outlineColor: string;
	boxShadow: string;
}

interface Stop {
	/** Unique per element, for cycle detection. */
	key: number;
	selector: string;
	before: Style | null;
	after: Style;
	background: string;
}

/** Prepares the page: every element's indicator styles are recorded before each Tab. */
async function install(page: Page) {
	await page.evaluate((carriers) => {
		const w = window as unknown as {
			__styles: WeakMap<Element, unknown>;
			__snapshot: () => void;
			__stop: () => unknown;
			__keys: WeakMap<Element, number>;
		};
		w.__keys = new WeakMap();
		let nextKey = 0;
		const style = (el: Element) => {
			const s = getComputedStyle(el);
			return {
				outlineStyle: s.outlineStyle,
				outlineWidth: s.outlineWidth,
				outlineColor: s.outlineColor,
				boxShadow: s.boxShadow,
			};
		};
		const carrierOf = (el: Element) => {
			for (const [match, carrier] of carriers) {
				if (el.matches(match)) {
					const c = el.closest(carrier);
					if (!c) throw new Error(`no carrier ${carrier} for ${match}`);
					return c;
				}
			}
			return el;
		};
		const describe = (el: Element) => {
			const id = el.id ? `#${el.id}` : "";
			const name = el.getAttribute("aria-label") ?? el.textContent?.trim().slice(0, 30) ?? "";
			return `${el.tagName.toLowerCase()}${id}[${el.getAttribute("role") ?? ""}] "${name}"`;
		};
		w.__snapshot = () => {
			w.__styles = new WeakMap();
			for (const el of document.querySelectorAll("body *")) w.__styles.set(el, style(el));
		};
		w.__stop = () => {
			const el = document.activeElement;
			if (!el || el === document.body) return null;
			const carrier = carrierOf(el);
			// Adjacent background: first ancestor of the carrier with an opaque one.
			let background = "rgb(255, 255, 255)";
			for (let a = carrier.parentElement; a; a = a.parentElement) {
				const bg = getComputedStyle(a).backgroundColor;
				const alpha = /\/\s*([\d.]+)\)|rgba\([^)]*,\s*([\d.]+)\)/.exec(bg);
				if (bg !== "rgba(0, 0, 0, 0)" && bg !== "transparent" && !alpha) {
					background = bg;
					break;
				}
			}
			if (!w.__keys.has(el)) w.__keys.set(el, nextKey++);
			return {
				key: w.__keys.get(el),
				selector: `${describe(el)}${carrier === el ? "" : ` via ${describe(carrier)}`}`,
				before: w.__styles.get(carrier) ?? null,
				after: style(carrier),
				background,
			};
		};
	}, CARRIERS);
}

/** Splits a computed `box-shadow` into [colour, spread] pairs. */
function shadows(value: string): { color: string; spread: number }[] {
	if (value === "none") return [];
	const parts: string[] = [];
	let depth = 0;
	let current = "";
	for (const ch of value) {
		if (ch === "(") depth++;
		if (ch === ")") depth--;
		if (ch === "," && depth === 0) {
			parts.push(current.trim());
			current = "";
		} else current += ch;
	}
	parts.push(current.trim());
	return parts.map((p) => {
		const color =
			/^(?:[a-z]+\([^)]*\)|#[0-9a-f]+)|(?:[a-z]+\([^)]*\)|#[0-9a-f]+)$/i.exec(p)?.[0] ?? "";
		const lengths = p
			.replace(color, "")
			.trim()
			.split(/\s+/)
			.filter((x) => x.endsWith("px"));
		return { color, spread: parseFloat(lengths[3] ?? "0") };
	});
}

/** The indicator drawn on focus, or why there is none. */
function check(stop: Stop): string | null {
	const { after, before } = stop;
	const outline =
		after.outlineStyle !== "none" && parseFloat(after.outlineWidth) > 0
			? after.outlineColor
			: null;
	const ring = shadows(after.boxShadow).find(
		(s) =>
			s.spread > 0 &&
			parseColor(s.color).a === 1 &&
			!shadows(before?.boxShadow ?? "none").some(
				(b) => b.color === s.color && b.spread === s.spread,
			),
	);
	let color: string | null = null;
	if (outline && parseColor(outline).a === 1) {
		const unchanged =
			before &&
			before.outlineStyle === after.outlineStyle &&
			before.outlineWidth === after.outlineWidth &&
			before.outlineColor === after.outlineColor;
		if (!unchanged) color = outline;
	}
	color ??= ring?.color ?? null;
	if (!color) return `no opaque outline or ring that appears on focus (${JSON.stringify(after)})`;
	const ratio = contrastRatio(color, stop.background);
	if (ratio < 3) return `indicator ${color} on ${stop.background}: ${ratio.toFixed(2)}:1`;
	return null;
}

async function tabThrough(page: Page, max = 60, minStops = 3) {
	await install(page);
	// Start unfocused, so a single-stop dialog is also seen gaining focus.
	await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
	const failures: string[] = [];
	const seen = new Map<number, string>();
	for (let i = 0; i < max; i++) {
		await page.evaluate(() => (window as unknown as { __snapshot: () => void }).__snapshot());
		await page.keyboard.press("Tab");
		// Measure the end state of any transition the focus started.
		await page.evaluate(() =>
			Promise.allSettled(document.getAnimations().map((a) => a.finished)),
		);
		const stop = (await page.evaluate(() =>
			(window as unknown as { __stop: () => unknown }).__stop(),
		)) as Stop | null;
		if (!stop) continue;
		if (seen.has(stop.key)) break;
		seen.set(stop.key, stop.selector);
		const problem = check(stop);
		if (problem) failures.push(`${stop.selector}: ${problem}`);
	}
	expect(seen.size, `tab stops: ${[...seen.values()].join(" | ")}`).toBeGreaterThanOrEqual(
		minStops,
	);
	expect(failures).toEqual([]);
	return [...seen.values()];
}

/** The indicator of the element that `action` focuses (it may not exist before). */
async function focusedBy(page: Page, action: () => Promise<void>) {
	await install(page);
	await page.evaluate(() => (window as unknown as { __snapshot: () => void }).__snapshot());
	await action();
	await page.evaluate(() => Promise.allSettled(document.getAnimations().map((a) => a.finished)));
	const stop = (await page.evaluate(() =>
		(window as unknown as { __stop: () => unknown }).__stop(),
	)) as Stop | null;
	expect(stop, "something has the focus").not.toBeNull();
	expect(stop && check(stop), stop?.selector).toBeNull();
}

test.describe("focus is visible on every tab stop", () => {
	test("unlock", async ({ page }) => {
		await openApp(page);
		await tabThrough(page);
	});

	test("create", async ({ page }) => {
		await openApp(page);
		await page.getByRole("button", { name: "Create new database" }).click();
		await page.getByRole("button", { name: "Advanced encryption settings" }).click();
		await tabThrough(page);
	});

	test("vault with an entry", async ({ page }) => {
		await unlock(page);
		await page.getByRole("option", { name: /GitHub/ }).click();
		await tabThrough(page);
	});

	test("entry editor", async ({ page }) => {
		await unlock(page);
		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.getByRole("button", { name: "Edit" }).click();
		await tabThrough(page);
	});

	for (const tab of ["General", "Database", "About"]) {
		test(`settings: ${tab}`, async ({ page }) => {
			await unlock(page);
			await page.keyboard.press("Control+,");
			await page.getByRole("tab", { name: tab }).click();
			await tabThrough(page);
		});
	}

	// Overlays and transient controls (M4A-R03).
	test("toast", async ({ page }) => {
		await unlock(page);
		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.getByRole("button", { name: "Copy username" }).click();
		await expect(page.getByTestId("toast")).toBeVisible();
		const stops = await tabThrough(page);
		expect(stops.some((s) => s.includes('"Dismiss"'))).toBe(true);
	});

	test("generator and keyboard shortcuts", async ({ page }) => {
		await unlock(page);
		await page.keyboard.press("Control+g");
		await expect(page.getByRole("dialog", { name: "Password generator" })).toBeVisible();
		await tabThrough(page, 20);
		await page.keyboard.press("Escape");
		await page.keyboard.press("Control+/");
		await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
		await tabThrough(page, 20, 1);
	});

	test("banners", async ({ page }) => {
		await unlock(page);
		await palette(page, "Demo: simulate a change by another app");
		await expect(page.getByRole("alert")).toContainText("changed by another app");
		let stops = await tabThrough(page);
		expect(stops.some((s) => s.includes('"Keep mine"'))).toBe(true);
		await page.getByRole("button", { name: "Reload" }).click();
		await palette(page, "Demo: simulate a save failure");
		await expect(page.getByRole("alert")).toContainText("Couldn’t save");
		stops = await tabThrough(page);
		expect(stops.some((s) => s.includes('"Retry"'))).toBe(true);
	});

	test("group name fields", async ({ page }) => {
		await unlock(page);
		const work = page.getByRole("treeitem", { name: /^Work/ });
		await work.click();
		await focusedBy(page, () => page.keyboard.press("F2"));
		await expect(page.getByRole("textbox", { name: "Rename group" })).toBeFocused();
		await page.keyboard.press("Escape");
		await focusedBy(page, () => page.getByRole("button", { name: "New group" }).click());
		await expect(page.getByRole("textbox", { name: "New group" })).toBeFocused();
	});

	test("command palette and confirm dialog", async ({ page }) => {
		await unlock(page);
		await page.keyboard.press("Control+k");
		// Arrow keys move through the results: the field is the only stop.
		await tabThrough(page, 10, 1);
		await page.keyboard.press("Escape");
		await expect(page.getByRole("dialog", { name: "Command palette" })).toBeHidden();
		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.keyboard.press("Delete");
		await tabThrough(page, 10, 2);
	});
});
