/**
 * WCAG 2.x contrast for CSS colours, including the `oklch()` / `oklab()`
 * values Tailwind v4 and Chromium produce. Shared by the token test
 * (`src/tests/contrast.test.ts`) and the focus check of the Playwright audit
 * (`e2e-web/keyboard.spec.ts`), so both measure colours the same way.
 */

/** Gamma-encoded sRGB channels in 0..1, plus alpha. */
export interface Rgba {
	r: number;
	g: number;
	b: number;
	a: number;
}

const NUMBER = String.raw`[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?`;

function channel(raw: string, scale: number): number {
	const value = parseFloat(raw);
	return raw.endsWith("%") ? (value / 100) * scale : value;
}

function alpha(raw: string | undefined): number {
	if (raw === undefined) return 1;
	return raw.endsWith("%") ? parseFloat(raw) / 100 : parseFloat(raw);
}

function encode(linear: number): number {
	const c = Math.min(1, Math.max(0, linear));
	return c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
}

function decode(encoded: number): number {
	return encoded <= 0.04045 ? encoded / 12.92 : ((encoded + 0.055) / 1.055) ** 2.4;
}

/** OKLab → gamma-encoded sRGB (clipped to the sRGB gamut). */
export function oklabToRgb(L: number, a: number, b: number): Omit<Rgba, "a"> {
	const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
	const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
	const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
	return {
		r: encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
		g: encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
		b: encode(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
	};
}

/**
 * Parses `#rgb[a]`, `#rrggbb[aa]`, `rgb()/rgba()`, `oklch()`, `oklab()` and
 * `color(srgb …)`. Throws on anything else, so an unexpected computed value
 * fails loudly instead of being measured wrong.
 */
export function parseColor(css: string): Rgba {
	const text = css.trim().toLowerCase();
	if (text === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
	const hex = /^#([0-9a-f]{3,8})$/.exec(text);
	if (hex) {
		let h = hex[1];
		if (h.length <= 4) h = [...h].map((c) => c + c).join("");
		const n = (i: number) => parseInt(h.slice(i, i + 2), 16) / 255;
		return { r: n(0), g: n(2), b: n(4), a: h.length === 8 ? n(6) : 1 };
	}
	const fn = /^([a-z]+)\((.*)\)$/.exec(text);
	if (!fn) throw new Error(`unsupported colour: ${css}`);
	const [, name, body] = fn;
	const [main, slash] = body.split("/").map((s) => s.trim());
	const parts = main
		.replace(/,/g, " ")
		.split(/\s+/)
		.filter(Boolean)
		.map((p) => (p === "none" ? "0" : p));
	const valid = new RegExp(`^${NUMBER}(%|deg)?$`);
	const values = name === "color" ? parts.slice(1) : parts;
	if (!values.every((p) => valid.test(p))) throw new Error(`unsupported colour: ${css}`);
	switch (name) {
		case "rgb":
		case "rgba": {
			const a = slash ?? parts[3];
			return {
				r: channel(parts[0], 255) / 255,
				g: channel(parts[1], 255) / 255,
				b: channel(parts[2], 255) / 255,
				a: alpha(a),
			};
		}
		case "oklch": {
			const L = channel(parts[0], 1);
			const C = channel(parts[1], 0.4);
			const h = (parseFloat(parts[2]) * Math.PI) / 180;
			return { ...oklabToRgb(L, C * Math.cos(h), C * Math.sin(h)), a: alpha(slash) };
		}
		case "oklab":
			return {
				...oklabToRgb(channel(parts[0], 1), channel(parts[1], 0.4), channel(parts[2], 0.4)),
				a: alpha(slash),
			};
		case "color":
			if (parts[0] !== "srgb") throw new Error(`unsupported colour space: ${css}`);
			return {
				r: channel(parts[1], 1),
				g: channel(parts[2], 1),
				b: channel(parts[3], 1),
				a: alpha(slash),
			};
		default:
			throw new Error(`unsupported colour: ${css}`);
	}
}

/** `top` painted over an opaque `bottom` (simple alpha compositing in sRGB). */
export function over(top: Rgba, bottom: Rgba): Rgba {
	const a = top.a;
	return {
		r: top.r * a + bottom.r * (1 - a),
		g: top.g * a + bottom.g * (1 - a),
		b: top.b * a + bottom.b * (1 - a),
		a: 1,
	};
}

/** WCAG relative luminance. */
export function luminance(c: Rgba): number {
	return 0.2126 * decode(c.r) + 0.7152 * decode(c.g) + 0.0722 * decode(c.b);
}

/**
 * WCAG contrast ratio of `fg` on `bg`. A translucent foreground is first
 * composited over the background; the background must be opaque.
 */
export function contrastRatio(fg: Rgba | string, bg: Rgba | string): number {
	const back = typeof bg === "string" ? parseColor(bg) : bg;
	if (back.a < 1) throw new Error("the background must be opaque");
	const front = over(typeof fg === "string" ? parseColor(fg) : fg, back);
	const [hi, lo] = [luminance(front), luminance(back)].sort((x, y) => y - x);
	return (hi + 0.05) / (lo + 0.05);
}
