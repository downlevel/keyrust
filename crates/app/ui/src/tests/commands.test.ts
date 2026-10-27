// @vitest-environment node
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import registry from "$lib/commands/commands.json";
import { formatShortcut, matchesShortcut, parseShortcut } from "$lib/commands/commands";

const messages: Record<string, string> = JSON.parse(
	readFileSync(new URL("../../messages/en.json", import.meta.url), "utf8"),
);

function key(init: Partial<KeyboardEvent> & { key: string }) {
	return {
		ctrlKey: false,
		metaKey: false,
		altKey: false,
		shiftKey: false,
		...init,
	} as KeyboardEvent;
}

describe("command registry", () => {
	it("has unique ids and translated labels", () => {
		const ids = registry.commands.map((c) => c.id);
		expect(new Set(ids).size).toBe(ids.length);
		for (const c of registry.commands) expect(messages[c.labelKey], c.id).toBeTypeOf("string");
		for (const menu of registry.menus)
			expect(messages[menu.labelKey], menu.id).toBeTypeOf("string");
	});

	it("only refers to declared menus", () => {
		const menus = new Set(registry.menus.map((m) => m.id));
		for (const c of registry.commands) if (c.menu) expect(menus.has(c.menu), c.id).toBe(true);
	});

	it("never binds a native accelerator that text editing needs", () => {
		const textKeys = [
			"CmdOrCtrl+C",
			"CmdOrCtrl+V",
			"CmdOrCtrl+X",
			"CmdOrCtrl+A",
			"CmdOrCtrl+Z",
			"Delete",
			"Enter",
		];
		for (const c of registry.commands) {
			if (c.nativeAccelerator) expect(textKeys, c.id).not.toContain(c.shortcut);
		}
	});

	it("binds Copy Password to the copy event, not to keydown", () => {
		const copy = registry.commands.find((c) => c.id === "copy_password");
		expect(copy).toMatchObject({ trigger: "copy", nativeAccelerator: false });
	});
});

describe("shortcuts", () => {
	it("parses accelerators", () => {
		expect(parseShortcut("CmdOrCtrl+Shift+N")).toEqual({
			primary: true,
			shift: true,
			alt: false,
			key: "n",
		});
		expect(parseShortcut("CmdOrCtrl+Slash").key).toBe("/");
		expect(parseShortcut("Delete").key).toBe("Delete");
	});

	it("maps CmdOrCtrl to Ctrl outside macOS and ⌘ on macOS", () => {
		expect(matchesShortcut(key({ key: "n", ctrlKey: true }), "CmdOrCtrl+N", false)).toBe(true);
		expect(matchesShortcut(key({ key: "n", metaKey: true }), "CmdOrCtrl+N", false)).toBe(false);
		expect(matchesShortcut(key({ key: "n", metaKey: true }), "CmdOrCtrl+N", true)).toBe(true);
		expect(matchesShortcut(key({ key: "n", ctrlKey: true }), "CmdOrCtrl+N", true)).toBe(false);
	});

	it("requires Shift exactly for letters, not for symbols", () => {
		expect(
			matchesShortcut(key({ key: "N", ctrlKey: true, shiftKey: true }), "CmdOrCtrl+N", false),
		).toBe(false);
		expect(
			matchesShortcut(
				key({ key: "N", ctrlKey: true, shiftKey: true }),
				"CmdOrCtrl+Shift+N",
				false,
			),
		).toBe(true);
		// Italian layout: "/" is Shift+7.
		expect(
			matchesShortcut(
				key({ key: "/", ctrlKey: true, shiftKey: true }),
				"CmdOrCtrl+Slash",
				false,
			),
		).toBe(true);
	});

	it("formats shortcuts for each platform", () => {
		expect(formatShortcut("CmdOrCtrl+Shift+N", false)).toBe("Ctrl+Shift+N");
		expect(formatShortcut("CmdOrCtrl+Shift+N", true)).toBe("⇧⌘N");
		expect(formatShortcut("Delete", false)).toBe("Del");
		expect(formatShortcut("CmdOrCtrl+Slash", false)).toBe("Ctrl+/");
	});
});
