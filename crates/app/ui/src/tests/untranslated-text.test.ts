// @vitest-environment node
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Heuristic check that visible text in application markup goes through m.*():
// after removing <script>/<style> blocks, comments, {expressions} and tags,
// nothing but whitespace may remain. It does not inspect attributes
// (aria-label, placeholder…). Vendored shadcn-svelte components in
// src/lib/components/ui are not application text and are skipped.
const roots = ["../routes", "../lib/screens", "../lib/components/app", "../lib/App.svelte"].map(
	(p) => fileURLToPath(new URL(p, import.meta.url)),
);

function svelteFiles(dir: string): string[] {
	if (dir.endsWith(".svelte")) return [dir];
	return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
		e.isDirectory()
			? svelteFiles(join(dir, e.name))
			: e.name.endsWith(".svelte")
				? [join(dir, e.name)]
				: [],
	);
}

function visibleText(markup: string): string {
	let text = markup
		.replace(/<script[\s\S]*?<\/script>/g, "")
		.replace(/<style[\s\S]*?<\/style>/g, "")
		.replace(/<!--[\s\S]*?-->/g, "");
	// Strip {expressions} from the innermost out, so nested braces go too.
	for (let previous = ""; previous !== text;) {
		previous = text;
		text = text.replace(/\{[^{}]*\}/g, "");
	}
	return text.replace(/<[^>]*>/g, "").trim();
}

describe("application markup", () => {
	const files = roots.flatMap(svelteFiles);

	it("finds the application files", () => {
		expect(files.length).toBeGreaterThan(20);
	});

	it.each(files)("%s has no literal visible text", (file) => {
		expect(visibleText(readFileSync(file, "utf8"))).toBe("");
	});

	it("detects literal text (self-test)", () => {
		expect(visibleText("<p>Hello</p>")).toBe("Hello");
		expect(visibleText("<p>{m.hello()}</p>")).toBe("");
		expect(visibleText("<p>{x ? m.a({ x }) : m.b()}</p>")).toBe("");
	});
});
