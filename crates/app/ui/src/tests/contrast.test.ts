import { readFileSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { describe, expect, it } from "vitest";
import { contrastRatio, parseColor } from "$lib/a11y/contrast";

// Colour tokens as the app defines them: `:root` in app.css, resolved against
// Tailwind's palette. Nothing is copied by hand, so changing a token changes
// what this test measures.

// Vitest runs from crates/app/ui (jsdom gives `import.meta.url` no file path).
const themeCss = readFileSync(resolvePath("node_modules/tailwindcss/theme.css"), "utf8");
const appCss = readFileSync(resolvePath("src/app.css"), "utf8");

function declarations(css: string): Map<string, string> {
	const out = new Map<string, string>();
	for (const [, name, value] of css.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
		if (!out.has(name)) out.set(name, value.trim());
	}
	return out;
}

const palette = declarations(themeCss);
const rootBlock = /:root\s*\{([\s\S]*?)\n\}/.exec(appCss)?.[1] ?? "";
const tokens = declarations(rootBlock);

function resolve(value: string, seen: string[] = []): string {
	const ref = /^var\((--[\w-]+)\)$/.exec(value);
	if (!ref) return value;
	const name = ref[1];
	if (seen.includes(name)) throw new Error(`cycle: ${[...seen, name].join(" → ")}`);
	const next = tokens.get(name) ?? palette.get(name);
	if (!next) throw new Error(`undefined variable ${name}`);
	return resolve(next, [...seen, name]);
}

/** The colour of a token from app.css, e.g. `muted-foreground`. */
function token(name: string): string {
	const value = tokens.get(`--${name}`);
	if (!value) throw new Error(`no token --${name} in app.css`);
	return resolve(value);
}

const TEXT = 4.5;
const LARGE_TEXT = 3;
const NON_TEXT = 3;

/** [foreground, background, minimum ratio, where it is used] */
const PAIRS: [string, string, number, string][] = [
	// Body text and secondary text on every surface they appear on.
	...["background", "surface", "sidebar", "card", "popover", "muted", "selection"].map(
		(bg) => ["foreground", bg, TEXT, "body text"] as [string, string, number, string],
	),
	...["background", "surface", "sidebar", "card", "popover", "muted", "accent", "selection"].map(
		(bg) =>
			["muted-foreground", bg, TEXT, "secondary text, counts, hints"] as [
				string,
				string,
				number,
				string,
			],
	),
	["sidebar-foreground", "sidebar", TEXT, "group names"],
	["sidebar-foreground", "muted", TEXT, "group names on hover"],
	["selection-foreground", "selection", TEXT, "selected group"],
	["accent-foreground", "accent", TEXT, "palette highlighted item"],
	["secondary-foreground", "secondary", TEXT, "secondary buttons"],
	// Links and text buttons.
	["primary", "background", TEXT, "links, New entry"],
	["primary", "surface", TEXT, "links on the Unlock screen"],
	["primary", "muted", TEXT, "New entry on hover"],
	["primary-hover", "background", TEXT, "links on hover"],
	["primary-hover", "surface", TEXT, "links on hover"],
	// Filled buttons.
	["primary-foreground", "primary", TEXT, "primary button"],
	["primary-foreground", "primary-hover", TEXT, "primary button on hover"],
	["destructive-foreground", "destructive", TEXT, "destructive button"],
	["destructive-foreground", "destructive-hover", TEXT, "destructive button on hover"],
	["primary-foreground", "warning-strong", TEXT, "Reload in the external-change banner"],
	// Messages.
	["destructive", "background", TEXT, "error text"],
	["destructive", "card", TEXT, "error text on the Unlock card"],
	["destructive-soft-foreground", "destructive-soft", TEXT, "save-failed banner"],
	["warning-soft-foreground", "warning-soft", TEXT, "external-change banner"],
	["warning-soft-foreground", "background", TEXT, "Caps Lock warning, Fair strength"],
	["success-text", "background", TEXT, "Passwords match, Good / Strong"],
	["foreground", "surface", TEXT, "KDBX conversion notice"],
	["toast-foreground", "toast", TEXT, "toast"],
	// Entry initials.
	...[0, 1, 2, 3, 4, 5].map(
		(i) =>
			[`avatar-${i}-fg`, `avatar-${i}-bg`, TEXT, "entry initials"] as [
				string,
				string,
				number,
				string,
			],
	),
	// Large text (the detail header initials are 18 px semibold: normal text).
	["primary", "selection", LARGE_TEXT, "empty-vault icon"],
	// Non-text: component boundaries, focus indicator, icons that carry meaning.
	["input", "background", NON_TEXT, "input borders"],
	["input", "surface", NON_TEXT, "search field border"],
	["input", "card", NON_TEXT, "input borders on cards"],
	...["background", "surface", "sidebar", "card", "selection", "muted"].map(
		(bg) => ["ring", bg, NON_TEXT, "focus indicator"] as [string, string, number, string],
	),
	...[1, 2, 3, 4].map(
		(i) =>
			[`strength-${i}`, "background", NON_TEXT, "strength meter segments"] as [
				string,
				string,
				number,
				string,
			],
	),
	["strength-1", "popover", NON_TEXT, "strength meter in the generator"],
	["strength-2", "popover", NON_TEXT, "strength meter in the generator"],
	["toast-icon", "toast", NON_TEXT, "toast check mark"],
	["warning-strong", "warning-soft", NON_TEXT, "banner warning icon"],
	["destructive", "destructive-soft", NON_TEXT, "banner error icon"],
	["primary", "background", NON_TEXT, "selected entry bar, switches on"],
];

describe("colour conversion", () => {
	it("matches known sRGB values", () => {
		const white = parseColor("oklch(1 0 0)");
		expect(white.r).toBeCloseTo(1, 3);
		expect(contrastRatio("#000", "#fff")).toBeCloseTo(21, 5);
		expect(contrastRatio("#777", "#fff")).toBeCloseTo(4.48, 2);
		// Tailwind's indigo-600 is about #4f39f6.
		const indigo = parseColor("oklch(51.1% 0.262 276.966)");
		expect(Math.round(indigo.r * 255)).toBeGreaterThanOrEqual(78);
		expect(Math.round(indigo.r * 255)).toBeLessThanOrEqual(80);
		expect(Math.round(indigo.b * 255)).toBeGreaterThanOrEqual(245);
		expect(parseColor("rgba(79, 57, 246, 0.5)").a).toBe(0.5);
		expect(parseColor("color(srgb 1 0.5 0 / 50%)").g).toBe(0.5);
		expect(() => parseColor("hsl(0 0% 0%)")).toThrow();
	});

	it("composites translucent colours over the background", () => {
		const half = contrastRatio("rgba(0, 0, 0, 0.5)", "#fff");
		expect(half).toBeGreaterThan(3);
		expect(half).toBeLessThan(contrastRatio("#000", "#fff"));
	});
});

describe("token contrast (WCAG 2.1 AA)", () => {
	it.each(PAIRS)("%s on %s ≥ %d:1 (%s)", (fg, bg, min) => {
		const ratio = contrastRatio(token(fg), token(bg));
		expect(ratio, `${fg} on ${bg}: ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(min);
	});
});
