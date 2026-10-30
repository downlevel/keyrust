import { expect, test } from "@playwright/test";
import { audit, openApp, palette, unlock } from "./helpers";

// Every screen and overlay of the app, in each state a user can reach.

test.describe("unlock and create", () => {
	test("unlock with recent files", async ({ page }, info) => {
		await openApp(page);
		await audit(page, info, "unlock");
	});

	test("unlock after a wrong password", async ({ page }, info) => {
		await openApp(page);
		await page.getByLabel("Master password").fill("wrong");
		await page.getByRole("button", { name: "Unlock" }).click();
		await expect(page.getByText("Wrong password or key.", { exact: false })).toBeVisible();
		await audit(page, info, "unlock-wrong-password");
	});

	test("welcome (no recent files)", async ({ page }, info) => {
		await openApp(page);
		await palette(page, "Demo: show the first-run screen");
		await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
		await audit(page, info, "welcome");
	});

	test("create database, with advanced settings and an error", async ({ page }, info) => {
		await openApp(page);
		await page.getByRole("button", { name: "Create new database" }).click();
		await expect(page.getByRole("heading", { name: "Create a new database" })).toBeVisible();
		await audit(page, info, "create");

		await page.getByLabel("Master password", { exact: true }).fill("correct horse battery");
		await page.getByLabel("Confirm master password").fill("correct horse");
		await page.getByRole("button", { name: "Advanced encryption settings" }).click();
		// The benchmark started on mount keeps its button disabled (half opacity) until it
		// ends; auditing while it fades back in measures a transitional colour.
		await expect(page.getByText(/^Unlocking takes about 1 second/)).toBeVisible();
		await expect(page.getByRole("button", { name: "Benchmark (1 s)" })).toBeEnabled();
		await audit(page, info, "create-advanced-mismatch");
	});
});

test.describe("vault", () => {
	test.beforeEach(async ({ page }) => unlock(page));

	test("list", async ({ page }, info) => {
		await audit(page, info, "vault-list");
	});

	test("selected entry, revealed password, toast", async ({ page }, info) => {
		await page.getByRole("option", { name: /GitHub/ }).click();
		await expect(page.getByRole("heading", { name: "GitHub" })).toBeVisible();
		await audit(page, info, "vault-entry");
		await page.getByRole("button", { name: "Copy username" }).click();
		await expect(page.getByTestId("toast")).toBeVisible();
		await audit(page, info, "vault-toast");
	});

	test("search without results", async ({ page }, info) => {
		await page.getByRole("searchbox").fill("nothing matches this");
		await expect(page.getByText("No matches for", { exact: false })).toBeVisible();
		await audit(page, info, "vault-search-empty");
	});

	test("recycle bin and an empty group", async ({ page }, info) => {
		await page.getByRole("treeitem", { name: /Recycle Bin/ }).click();
		await audit(page, info, "vault-recycle-bin");
		await page.getByRole("button", { name: "New group" }).click();
		await page.getByRole("textbox", { name: "New group" }).fill("Empty");
		await page.keyboard.press("Enter");
		await page.getByRole("treeitem", { name: /Empty/ }).click();
		await audit(page, info, "vault-empty-group");
	});

	test("entry editor: new, edit, generator popover", async ({ page }, info) => {
		await page.getByRole("button", { name: "New entry" }).first().click();
		await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
		await audit(page, info, "editor-new");
		await page.keyboard.press("Escape");

		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.getByRole("button", { name: "Edit" }).click();
		await expect(page.getByLabel("Title")).toBeVisible();
		await audit(page, info, "editor-edit");
		await page.getByRole("button", { name: "Generate", exact: true }).click();
		await expect(page.getByRole("dialog", { name: "Password generator" })).toBeVisible();
		await audit(page, info, "editor-generator");
	});

	test("settings, every tab", async ({ page }, info) => {
		await page.keyboard.press("Control+,");
		for (const tab of ["General", "Database", "About"]) {
			await page.getByRole("tab", { name: tab }).click();
			await expect(page.getByRole("tab", { name: tab })).toHaveAttribute(
				"aria-selected",
				"true",
			);
			await audit(page, info, `settings-${tab.toLowerCase()}`);
		}
	});

	test("keyboard shortcuts panel and command palette", async ({ page }, info) => {
		await page.keyboard.press("Control+/");
		await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
		await audit(page, info, "shortcuts");
		await page.keyboard.press("Escape");
		await page.keyboard.press("Control+k");
		await expect(page.getByRole("combobox", { name: "Command palette" })).toBeFocused();
		await audit(page, info, "palette");
	});

	test("standalone generator", async ({ page }, info) => {
		await page.keyboard.press("Control+g");
		await expect(page.getByRole("dialog", { name: "Password generator" })).toBeVisible();
		await audit(page, info, "generator");
	});

	test("confirm dialog", async ({ page }, info) => {
		await page.getByRole("option", { name: /GitHub/ }).click();
		await page.getByRole("button", { name: "Delete" }).click();
		await expect(page.getByRole("alertdialog")).toBeVisible();
		await audit(page, info, "confirm");
	});

	test("banners: external change and save failure", async ({ page }, info) => {
		await palette(page, "Demo: simulate a change by another app");
		await expect(page.getByRole("alert")).toContainText("changed by another app");
		await audit(page, info, "banner-external");
		await page.getByRole("button", { name: "Reload" }).click();
		await palette(page, "Demo: simulate a save failure");
		await expect(page.getByRole("alert")).toContainText("Couldn’t save");
		await audit(page, info, "banner-save-failed");
	});
	// The KDBX 3.1 / 4.0 notice cannot appear in the demo (always 4.1); its
	// tone is checked by the token contrast test and src/tests/version-notice.test.ts.
});

test.describe("narrow window (collapsed sidebar)", () => {
	test.use({ viewport: { width: 900, height: 720 } });

	test("saves are still announced, and the layout passes axe (M4A-R10)", async ({
		page,
	}, info) => {
		await unlock(page);
		await expect(page.getByRole("button", { name: "Expand sidebar" })).toBeVisible();
		const status = page.getByTestId("save-status");
		await expect(status).toHaveAttribute("role", "status");
		await page.keyboard.press("Control+s");
		await expect(status).toHaveText("Saved");
		await audit(page, info, "vault-narrow");
	});
});
