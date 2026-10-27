// @vitest-environment node
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Static guards on the frontend sources (tests and generated code excluded).
const src = fileURLToPath(new URL("..", import.meta.url));

function sources(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
		const path = join(dir, e.name);
		if (e.isDirectory()) return ["tests", "paraglide"].includes(e.name) ? [] : sources(path);
		return /\.(ts|js|svelte|css|html)$/.test(e.name) ? [path] : [];
	});
}

const files = sources(src).map((path) => ({ path, text: readFileSync(path, "utf8") }));

describe("frontend sources", () => {
	it("are found", () => {
		expect(files.length).toBeGreaterThan(30);
	});

	it("never use browser storage", () => {
		for (const { path, text } of files) {
			expect(text, path).not.toMatch(/\b(localStorage|sessionStorage|indexedDB)\b/);
		}
	});

	it("load no remote resources (only plain links to the project site)", () => {
		for (const { path, text } of files) {
			expect(text, path).not.toMatch(/\b(src|srcset)=["'{]?\s*["'`]?https?:/);
			expect(text, path).not.toMatch(/@import\s+(url\()?["']?https?:/);
			expect(text, path).not.toMatch(/\bfetch\(/);
			for (const [, url] of text.matchAll(/href="(https?:[^"]+)"/g)) {
				expect(url, path).toMatch(/^https:\/\/github\.com\/downlevel\/keyrust/);
			}
		}
	});

	it("keep secret inputs out of autocomplete and spellcheck", () => {
		const secret = files.find((f) => f.path.endsWith("SecretInput.svelte"))?.text ?? "";
		expect(secret).toMatch(/spellcheck=\{false\}/);
		expect(secret).toMatch(/autocomplete/);
	});
});
