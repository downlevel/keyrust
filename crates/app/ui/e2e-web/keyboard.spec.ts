import { expect, test, type Locator, type Page } from "@playwright/test";
import registry from "../src/lib/commands/commands.json" with { type: "json" };
import { openApp, palette, unlock } from "./helpers";

/** What the focused element is, for readable order assertions. */
async function focused(page: Page): Promise<string> {
	return page.evaluate(() => {
		const el = document.activeElement as HTMLElement | null;
		if (!el || el === document.body) return "body";
		const implicit: Record<string, string> = {
			button: "button",
			a: "link",
			textarea: "textbox",
		};
		const input =
			el instanceof HTMLInputElement
				? el.type === "search"
					? "searchbox"
					: "textbox"
				: null;
		const role =
			el.getAttribute("role") ??
			input ??
			implicit[el.tagName.toLowerCase()] ??
			el.tagName.toLowerCase();
		const name =
			el.getAttribute("aria-label") ??
			(el as HTMLInputElement).placeholder ??
			el.textContent?.trim().replace(/\s+/g, " ") ??
			"";
		return `${role}:${name}`.slice(0, 60);
	});
}

/** Tab stops from the current focus until it comes back (at most `max`). */
async function cycle(page: Page, key: "Tab" | "Shift+Tab", max = 60): Promise<string[]> {
	const start = await focused(page);
	const stops = [start];
	for (let i = 0; i < max; i++) {
		await page.keyboard.press(key);
		const now = await focused(page);
		if (now === start) return stops;
		stops.push(now);
	}
	throw new Error(`focus did not cycle within ${max} stops: ${stops.join(" | ")}`);
}

async function expectFocusInside(page: Page, container: Locator) {
	expect(await container.evaluate((el) => el.contains(document.activeElement))).toBe(true);
}

test.describe("vault tab order", () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.keyboard.press("Control+f");
		await expect(page.getByRole("searchbox")).toBeFocused();
	});

	test("the tree and the list are one stop each, in reading order", async ({ page }) => {
		const forward = await cycle(page, "Tab");
		const index = (prefix: string) => forward.findIndex((s) => s.startsWith(prefix));
		expect(forward.filter((s) => s.startsWith("treeitem:"))).toHaveLength(1);
		expect(forward.filter((s) => s.startsWith("option:"))).toHaveLength(1);
		// Search → list → detail actions, then round to the sidebar (tree) and back.
		expect(index("searchbox:")).toBe(0);
		expect(index("option:")).toBeGreaterThan(index("searchbox:"));
		expect(index("button:Delete")).toBeGreaterThan(index("option:"));
		expect(index("button:Copy password")).toBeGreaterThan(index("button:Delete"));
		expect(index("treeitem:")).toBeGreaterThan(index("button:Copy password"));
		// The selected entry and group are the stops.
		expect(forward[index("option:")]).toContain("GitHub");
		expect(forward[index("treeitem:")]).toContain("All entries");

		const backward = await cycle(page, "Shift+Tab");
		expect(backward).toEqual([forward[0], ...forward.slice(1).reverse()]);
	});
});

test.describe("tree and list keys", () => {
	test.beforeEach(async ({ page }) => unlock(page));

	test("arrows, Home and End move through the groups", async ({ page }) => {
		const tree = page.getByRole("tree", { name: "Groups" });
		await tree.getByRole("treeitem", { name: /^All entries/ }).focus();
		await page.keyboard.press("ArrowDown");
		await expect(tree.getByRole("treeitem", { name: /^Email/ })).toBeFocused();
		await expect(tree.getByRole("treeitem", { name: /^Email/ })).toHaveAttribute(
			"aria-selected",
			"true",
		);
		await page.keyboard.press("ArrowUp");
		await expect(tree.getByRole("treeitem", { name: /^All entries/ })).toBeFocused();
		await page.keyboard.press("End");
		await expect(tree.getByRole("treeitem", { name: /^Recycle Bin/ })).toBeFocused();
		await page.keyboard.press("Home");
		await expect(tree.getByRole("treeitem", { name: /^All entries/ })).toBeFocused();
		// Groups are never collapsed, so no node claims to be expandable.
		expect(await tree.locator("[aria-expanded]").count()).toBe(0);
		// Structure for screen readers: levels and positions among siblings.
		const dev = tree.getByRole("treeitem", { name: /^Dev/ });
		await expect(dev).toHaveAttribute("aria-level", "2");
		await expect(dev).toHaveAttribute("aria-setsize", "1");
		await expect(tree.getByRole("treeitem", { name: /^Work/ })).toHaveAccessibleName(
			"Work, 1 entry",
		);
	});

	test("End with no selection goes to the last entry (M4A-R02)", async ({ page }) => {
		const options = page.getByRole("listbox").getByRole("option");
		const count = await options.count();
		await options.first().focus();
		await page.keyboard.press("End");
		await expect(options.nth(count - 1)).toBeFocused();
		await expect(options.nth(count - 1)).toHaveAttribute("aria-selected", "true");
	});

	test("arrows, Home, End and Enter in the list", async ({ page }) => {
		const list = page.getByRole("listbox");
		await expect(list).toHaveAccessibleName("All entries");
		const options = list.getByRole("option");
		const count = await options.count();
		await options.first().focus();
		await page.keyboard.press("ArrowDown");
		await expect(options.nth(0)).toHaveAttribute("aria-selected", "true");
		await page.keyboard.press("ArrowDown");
		await expect(options.nth(1)).toBeFocused();
		await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
		await page.keyboard.press("End");
		await expect(options.nth(count - 1)).toBeFocused();
		await expect(options.nth(count - 1)).toHaveAttribute("aria-selected", "true");
		await page.keyboard.press("Home");
		await expect(options.nth(0)).toBeFocused();
		await page.keyboard.press("ArrowUp");
		await expect(options.nth(0)).toBeFocused();
		// One selected option at a time, roving tabindex (no aria-activedescendant).
		await expect(list.locator('[aria-selected="true"]')).toHaveCount(1);
		await expect(list.locator('[tabindex="0"]')).toHaveCount(1);
		expect(await list.getAttribute("aria-activedescendant")).toBeNull();
		await page.keyboard.press("Enter");
		await expect(page.getByLabel("Title")).toBeVisible();
	});

	test("the list is named after the group or the search", async ({ page }) => {
		await page.getByRole("searchbox").fill("git");
		await expect(page.getByRole("listbox")).toHaveAccessibleName("Entries matching “git”");
		// Announced once typing pauses.
		await expect(page.getByTestId("search-announcement")).toHaveText("2 entries");
		await page.getByRole("searchbox").fill("");
		await expect(page.getByTestId("search-announcement")).toHaveText("");
		await page.getByRole("treeitem", { name: /^Banking/ }).click();
		await expect(page.getByRole("listbox")).toHaveAccessibleName("Entries in Banking");
	});
});

test.describe("dialogs trap focus and give it back", () => {
	test.beforeEach(async ({ page }) => unlock(page));

	async function trapAndReturn(page: Page, dialog: Locator, opener: Locator) {
		await expect(dialog).toBeVisible();
		await expectFocusInside(page, dialog);
		for (let i = 0; i < 25; i++) {
			await page.keyboard.press("Tab");
			await expectFocusInside(page, dialog);
		}
		await page.keyboard.press("Escape");
		await expect(dialog).toBeHidden();
		await expect(opener).toBeFocused();
	}

	test("Settings", async ({ page }) => {
		const opener = page.getByRole("button", { name: "Settings" });
		await opener.focus();
		await page.keyboard.press("Enter");
		await trapAndReturn(page, page.getByRole("dialog", { name: "Settings" }), opener);
	});

	test("Confirm", async ({ page }) => {
		await page.getByRole("option", { name: /GitHub/ }).click();
		const opener = page.getByRole("button", { name: "Delete" });
		await opener.focus();
		await page.keyboard.press("Enter");
		await trapAndReturn(page, page.getByRole("alertdialog"), opener);
	});

	test("Command palette", async ({ page }) => {
		const opener = page.getByRole("searchbox");
		await opener.focus();
		await page.keyboard.press("Control+k");
		await trapAndReturn(page, page.getByRole("dialog", { name: "Command palette" }), opener);
	});
});

interface RegistryCommand {
	id: string;
	menu: string | null;
	shortcut: string | null;
	nativeAccelerator: boolean;
	demoOnly?: boolean;
}
const COMMANDS = registry.commands as RegistryCommand[];

/** Only the native menu can do these: the webview never sees the key. */
const NATIVE_ONLY = new Set(["quit"]);

function keys(shortcut: string): string {
	return shortcut
		.replace("CmdOrCtrl", "Control")
		.replace("Comma", ",")
		.replace("Slash", "/")
		.split("+")
		.map((k) => (k.length === 1 ? k.toLowerCase() : k))
		.join("+");
}

/** Each shortcut's observable effect, starting from an unlocked vault with GitHub selected. */
const EFFECTS: Record<string, (page: Page) => Promise<void>> = {
	new_database: (p) =>
		expect(p.getByRole("heading", { name: "Create a new database" })).toBeVisible(),
	open_database: (p) => expect(p.getByLabel("Master password")).toBeVisible(),
	save: (p) => expect(p.getByRole("alert")).toContainText("Couldn’t save"),
	lock: (p) => expect(p.getByLabel("Master password")).toBeVisible(),
	find: (p) => expect(p.getByRole("searchbox")).toBeFocused(),
	command_palette: (p) =>
		expect(p.getByRole("combobox", { name: "Command palette" })).toBeFocused(),
	new_entry: (p) => expect(p.getByRole("heading", { name: "New entry" })).toBeVisible(),
	edit_entry: (p) => expect(p.getByLabel("Title")).toHaveValue("GitHub"),
	copy_username: (p) => expect(p.getByTestId("toast")).toContainText("Username copied"),
	copy_password: (p) => expect(p.getByTestId("toast")).toContainText("Password copied"),
	copy_url: (p) => expect(p.getByTestId("toast")).toContainText("URL copied"),
	delete_entry: (p) => expect(p.getByRole("alertdialog")).toBeVisible(),
	rename_group: (p) => expect(p.getByRole("textbox", { name: "Rename group" })).toBeFocused(),
	password_generator: (p) =>
		expect(p.getByRole("dialog", { name: "Password generator" })).toBeVisible(),
	settings: (p) =>
		expect(p.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true"),
	keyboard_shortcuts: (p) =>
		expect(p.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible(),
};

test.describe("every documented shortcut works without the mouse", () => {
	const withShortcut = COMMANDS.filter((c) => c.shortcut && !NATIVE_ONLY.has(c.id));

	test("the list of effects covers the registry", () => {
		expect(Object.keys(EFFECTS).sort()).toEqual(withShortcut.map((c) => c.id).sort());
	});

	for (const c of withShortcut) {
		test(`${c.id} (${c.shortcut})`, async ({ page }) => {
			if (c.id === "save") {
				// Arm the demo's failure before unlocking: the failed save is visible.
				await openApp(page);
				await palette(page, "Demo: simulate a save failure");
				await page.getByLabel("Master password").fill("secret");
				await page.getByRole("button", { name: "Unlock" }).click();
				await expect(page.getByRole("listbox")).toBeVisible();
			} else {
				await unlock(page);
			}
			const github = page.getByRole("option", { name: /GitHub/ });
			await github.click();
			if (c.id === "rename_group") {
				await page.getByRole("treeitem", { name: /^Work/ }).click();
			}
			await page.keyboard.press(keys(c.shortcut ?? ""));
			await EFFECTS[c.id](page);
		});
	}

	test("menu commands without a shortcut are in the command palette", async ({ page }) => {
		await unlock(page);
		await page.keyboard.press("Control+k");
		const labels = await page.getByRole("option").allTextContents();
		const text = labels.join("\n");
		for (const id of [
			"close_database",
			"new_group",
			"delete_group",
			"database_settings",
			"about",
		]) {
			const c = COMMANDS.find((x) => x.id === id);
			expect(c?.shortcut, id).toBeNull();
		}
		for (const label of [
			"Close Database",
			"New Group",
			"Delete Group",
			"Database Settings",
			"About Keyrust",
		]) {
			expect(text).toContain(label);
		}
	});
});
