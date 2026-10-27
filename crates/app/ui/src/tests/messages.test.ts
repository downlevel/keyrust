// @vitest-environment node
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { m } from "$lib/paraglide/messages.js";

const source: Record<string, string> = JSON.parse(
	readFileSync(new URL("../../messages/en.json", import.meta.url), "utf8"),
);
const keys = Object.keys(source).filter((k) => !k.startsWith("$"));
const messages = m as unknown as Record<string, ((inputs?: object) => string) | undefined>;

describe("English messages", () => {
	it("compiles every key of messages/en.json", () => {
		// The Paraglide compiler reports success even when its plugin fails to
		// load and no message is generated: guard against that here.
		expect(keys.length).toBeGreaterThan(0);
		for (const key of keys) {
			expect(typeof messages[key], key).toBe("function");
		}
	});

	it("renders the source text for messages without parameters", () => {
		for (const key of keys.filter((k) => !source[k].includes("{"))) {
			expect(messages[key]?.(), key).toBe(source[key]);
		}
	});

	it("interpolates parameters", () => {
		expect(m.core_version({ version: "1.2.3" })).toBe("core v1.2.3");
	});
});
