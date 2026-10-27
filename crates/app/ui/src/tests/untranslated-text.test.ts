import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// Heuristic check that visible text in route markup goes through m.*():
// after removing <script>/<style> blocks, {expressions} and tags, nothing but
// whitespace may remain. It does not inspect attributes (aria-label,
// placeholder…) nor components outside src/routes.
const routes = new URL("../routes", import.meta.url).pathname;

function svelteFiles(dir: string): string[] {
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
		.replace(/<style[\s\S]*?<\/style>/g, "");
	// Strip {expressions} from the innermost out, so nested braces go too.
	for (let previous = ""; previous !== text;) {
		previous = text;
		text = text.replace(/\{[^{}]*\}/g, "");
	}
	return text.replace(/<[^>]*>/g, "").trim();
}

describe("route markup", () => {
	const files = svelteFiles(routes);

	it("finds the route files", () => {
		expect(files.length).toBeGreaterThan(0);
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
