import { $, browser, expect } from "@wdio/globals";
import { FIXTURE_PASSWORD, paths } from "../setup.ts";

// One run of the real app on copies of the KeePassXC fixtures
// (tests/fixtures/README.md). Waits are on conditions only, never fixed pauses.

/** Every password of keepassxc-rich.kdbx (current, history, protected custom field, bin). */
const SECRETS = [
	"gh-current-ω-3",
	"gh-old-1",
	"gh-old-2",
	"RC-1234-5678",
	"gm-p@ss",
	"old-pass",
	FIXTURE_PASSWORD,
];
const NEW_TITLE = "GitHub (e2e)";

/**
 * Looks for the passwords anywhere in the document: text of every node
 * (hidden ones and portals included), every attribute value and the current
 * value of every input and textarea.
 */
async function secretsInDom(): Promise<string[]> {
	return browser.execute((secrets: string[]) => {
		const found = new Set<string>();
		const check = (text: string | null | undefined, where: string) => {
			if (!text) return;
			for (const s of secrets)
				if (text.includes(s)) found.add(`${s} in ${where}`);
		};
		check(document.documentElement.textContent, "text");
		for (const el of document.querySelectorAll("*")) {
			for (const attr of el.attributes)
				check(attr.value, `@${attr.name} of <${el.tagName}>`);
			if (
				el instanceof HTMLInputElement ||
				el instanceof HTMLTextAreaElement
			) {
				check(el.value, `value of <${el.tagName}#${el.id}>`);
			}
		}
		return [...found];
	}, SECRETS);
}

/** Titles of the entries in the list, in order. */
async function optionTitles(): Promise<string[]> {
	return browser.execute(() =>
		[
			...document.querySelectorAll(
				'[role="listbox"] [role="option"] span.font-medium',
			),
		].map((el) => el.textContent?.trim() ?? ""),
	);
}

/**
 * Waits for an element's text. Read from the DOM: WebKitWebDriver's "Get
 * Element Text" returns "" for `truncate` (overflow hidden) elements.
 */
async function expectText(selector: string, expected: string) {
	const read = () =>
		browser.execute(
			(sel: string) =>
				document.querySelector(sel)?.textContent?.trim() ?? null,
			selector,
		);
	await browser.waitUntil(async () => (await read()) === expected, {
		timeoutMsg: `${selector}: expected "${expected}", found "${await read()}"`,
	});
}

function option(title: string) {
	return $(
		`//*[@role="listbox"]//*[@role="option"][.//span[normalize-space()="${title}"]]`,
	);
}

async function recentButton(stem: string) {
	return $(
		`//section[@aria-labelledby="recent-heading"]//button[.//span[normalize-space()="${stem}"]]`,
	);
}

async function unlockWith(password: string) {
	const field = await $("#master-password");
	await field.waitForEnabled();
	await field.setValue(password);
	await $('form button[type="submit"]').click();
}

describe("Keyrust smoke test", () => {
	it("lists the two databases from the recent files", async () => {
		const p = paths();
		const count = () =>
			browser.execute(
				() =>
					document.querySelectorAll(
						'section[aria-labelledby="recent-heading"] li button',
					).length,
			);
		// The first start of WebKitGTK in CI can be slow.
		await browser.waitUntil(async () => (await count()) > 0, {
			timeout: 60_000,
			timeoutMsg: `no recent files: is the settings file read from ${p.settings}?`,
		});
		expect(await count()).toBe(2);
		await expect(await recentButton("keepassxc-rich")).toBeDisplayed();
		await expect(await recentButton("keepassxc-kdbx31")).toBeDisplayed();
		await expectText('[data-testid="unlock-db-name"]', "keepassxc-rich");
	});

	it("refuses a wrong password and keeps the focus in the field", async () => {
		await unlockWith("not-the-password");
		const error = await $("#master-password-error");
		await error.waitUntil(async () => (await error.getText()).length > 0);
		expect(await error.getAttribute("role")).toBe("alert");
		await browser.waitUntil(
			async () =>
				(await browser.execute(() => document.activeElement?.id)) ===
				"master-password",
			{ timeoutMsg: "the password field lost the focus" },
		);
	});

	it("opens the vault with no password in the page", async () => {
		await unlockWith(FIXTURE_PASSWORD);
		await $('[role="listbox"]').waitForDisplayed();
		expect((await optionTitles()).sort()).toEqual(["GitHub", "Gmail"]);
		expect(await secretsInDom()).toEqual([]);

		await option("GitHub").click();
		await $("#entry-title").waitForDisplayed();
		await expectText('[data-testid="password-value"]', "••••••••••••");
		expect(await secretsInDom()).toEqual([]);
	});

	it("filters the list by search and restores it", async () => {
		const search = await $('input[type="search"]');
		await search.setValue("Gma");
		await browser.waitUntil(
			async () => (await optionTitles()).length === 1,
		);
		expect(await optionTitles()).toEqual(["Gmail"]);
		await $('button[aria-label="Clear search"]').click();
		await browser.waitUntil(
			async () => (await optionTitles()).length === 2,
		);
	});

	it("renames an entry and saves the file from the editor", async () => {
		await option("GitHub").click();
		await $('//article//header//button[normalize-space()="Edit"]').click();
		const title = await $("#editor-title");
		await title.waitForDisplayed();
		await title.clearValue();
		await title.setValue(NEW_TITLE);
		await $('form button[type="submit"]').click();
		await expectText("#entry-title", NEW_TITLE);
		await browser.waitUntil(
			async () =>
				(await $('[data-testid="save-status"]').getText()) ===
					"Saved" &&
				!(await $('[data-testid="unsaved-dot"]').isExisting()),
			{ timeoutMsg: "the save did not complete" },
		);
	});

	it("locks back to the Unlock screen", async () => {
		await $('button[aria-label="Lock database"]').click();
		await $("#master-password").waitForDisplayed();
		expect(await $('[role="listbox"]').isExisting()).toBe(false);
	});

	it("opens a KDBX 3.1 file with the conversion notice, without saving", async () => {
		await (await recentButton("keepassxc-kdbx31")).click();
		await expectText('[data-testid="unlock-db-name"]', "keepassxc-kdbx31");
		await unlockWith(FIXTURE_PASSWORD);
		const notice = await $(
			'//*[@role="status"][contains(., "This database uses KDBX 3.1")]',
		);
		await notice.waitForDisplayed();
		expect(await optionTitles()).toEqual(["Legacy"]);
	});
});
