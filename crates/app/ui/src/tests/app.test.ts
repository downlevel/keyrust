import { fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "$lib/App.svelte";
import { MemoryAppApi } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";

function setup() {
	const vault = new DemoVault({ delayMs: 0 });
	const user = userEvent.setup();
	render(App, { apis: { vault, app: new MemoryAppApi() } });
	return { user, vault };
}

async function unlock(user: ReturnType<typeof userEvent.setup>, password = "secret") {
	const input = await screen.findByLabelText("Master password");
	await user.type(input, password);
	await user.click(screen.getByRole("button", { name: "Unlock" }));
}

async function openVault() {
	const ctx = setup();
	await unlock(ctx.user);
	await screen.findByRole("listbox", { name: "Entries" });
	return ctx;
}

const option = (name: RegExp | string) =>
	within(screen.getByRole("listbox", { name: "Entries" })).getByRole("option", { name });

afterEach(() => {
	vi.useRealTimers();
});

describe("unlock", () => {
	it("shows the demo notice, rejects a wrong password and clears the field", async () => {
		const { user } = setup();
		expect(
			await screen.findByText("No file is opened and nothing is copied or saved."),
		).toBeInTheDocument();
		await unlock(user, "wrong");
		expect(
			await screen.findByText("Wrong password or key. Check Caps Lock and try again."),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Master password")).toHaveValue("");
		expect(screen.getByLabelText("Master password")).toHaveAttribute("aria-invalid", "true");
	});

	it("opens the three-column vault with accessible roles", async () => {
		await openVault();
		expect(screen.getByRole("tree", { name: "Groups" })).toBeInTheDocument();
		expect(screen.getByText("9 entries")).toBeInTheDocument();
		expect(screen.getByText("Select an entry to see its details.")).toBeInTheDocument();
		expect(document.querySelector('[aria-live="polite"]')).not.toBeNull();
		expect(screen.getAllByTestId("demo-badge").length).toBeGreaterThan(0);
		// With nothing selected the first entry is the list's tab stop (M2-R204).
		const options = screen.getAllByRole("option");
		expect(options[0]).toHaveAttribute("tabindex", "0");
		expect(options.slice(1).every((o) => o.getAttribute("tabindex") === "-1")).toBe(true);
	});
});

describe("vault", () => {
	it("searches titles, usernames and URLs and reports no matches", async () => {
		const { user } = await openVault();
		const search = screen.getByRole("searchbox", { name: "Search entries" });
		await user.type(search, "git");
		expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
			expect.stringContaining("GitHub"),
			expect.stringContaining("GitLab"),
		]);
		await user.clear(search);
		await user.type(search, "netflix");
		expect(screen.getByText("No matches for “netflix”")).toBeInTheDocument();
	});

	it("masks the password by default and hides it again on another entry", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		expect(await screen.findByRole("heading", { name: "GitHub" })).toBeInTheDocument();
		expect(screen.getByTestId("password-value")).toHaveTextContent("••••");
		await user.click(screen.getByRole("button", { name: "Reveal password" }));
		expect(screen.getByTestId("password-value")).toHaveTextContent("tW8#qLz!v2Rm9@kP4sXe");
		await user.click(option(/GitLab/));
		await screen.findByRole("heading", { name: "GitLab" });
		expect(screen.getByTestId("password-value")).toHaveTextContent("••••");
	});

	it("copy shows a toast that counts down and disappears", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		await screen.findByRole("heading", { name: "GitHub" });
		vi.useFakeTimers();
		await fireEvent.click(screen.getByRole("button", { name: "Copy password" }));
		await vi.waitFor(() =>
			expect(screen.getByText("Password copied · clears in 20s")).toBeInTheDocument(),
		);
		await vi.advanceTimersByTimeAsync(1000);
		expect(screen.getByText("Password copied · clears in 19s")).toBeInTheDocument();
		await vi.advanceTimersByTimeAsync(19000);
		expect(screen.queryByText(/copied · clears/)).not.toBeInTheDocument();
	});

	it("creates an entry, requiring a title", async () => {
		const { user } = await openVault();
		await user.click(screen.getByRole("button", { name: "New entry" }));
		await user.click(await screen.findByRole("button", { name: "Save" }));
		expect(screen.getByText("A title is required.")).toBeInTheDocument();
		await user.type(screen.getByLabelText("Title"), "Netflix");
		await user.type(screen.getByLabelText("Username"), "alex@example.com");
		await user.click(screen.getByRole("button", { name: "Save" }));
		expect(await screen.findByRole("heading", { name: "Netflix" })).toBeInTheDocument();
		expect(option(/Netflix/)).toHaveAttribute("aria-selected", "true");
	});

	it("asks before leaving a modified entry and honours each choice", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		await user.click(await screen.findByRole("button", { name: "Edit" }));
		const username = await screen.findByLabelText("Username");
		await user.clear(username);
		await user.type(username, "changed");
		expect(screen.getByTestId("unsaved-dot")).toBeInTheDocument();

		await user.click(option(/GitLab/));
		const dialog = await screen.findByRole("alertdialog");
		expect(within(dialog).getByText("Save changes to this entry?")).toBeInTheDocument();
		await user.click(within(dialog).getByRole("button", { name: "Keep editing" }));
		await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
		expect(screen.getByLabelText("Username")).toHaveValue("changed");

		await user.click(option(/GitLab/));
		await user.click(
			within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Discard" }),
		);
		expect(await screen.findByRole("heading", { name: "GitLab" })).toBeInTheDocument();
	});

	it("Esc on an unmodified editor closes it, on a modified one asks", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		await user.click(await screen.findByRole("button", { name: "Edit" }));
		await screen.findByLabelText("Title");
		await user.keyboard("{Escape}");
		expect(await screen.findByRole("heading", { name: "GitHub" })).toBeInTheDocument();
		await user.click(screen.getByRole("button", { name: "Edit" }));
		await user.type(await screen.findByLabelText("Notes"), "x");
		await user.keyboard("{Escape}");
		expect(await screen.findByRole("alertdialog")).toBeInTheDocument();
	});

	it("moves an entry to the recycle bin after confirmation", async () => {
		const { user } = await openVault();
		await user.click(option(/Reddit/));
		await screen.findByRole("heading", { name: "Reddit" });
		await user.keyboard("{Delete}");
		const dialog = await screen.findByRole("alertdialog");
		await user.click(within(dialog).getByRole("button", { name: "Move to Recycle Bin" }));
		await waitFor(() =>
			expect(screen.queryByRole("option", { name: /Reddit/ })).not.toBeInTheDocument(),
		);
		expect(screen.getByText("8 entries")).toBeInTheDocument();
	});

	it("generator keeps one character set and fills the password", async () => {
		const { user } = await openVault();
		await user.click(screen.getByRole("button", { name: "New entry" }));
		await user.click(await screen.findByRole("button", { name: /Generate/ }));
		const panel = await screen.findByTestId("password-generator");
		for (const name of [
			"Lowercase (a–z)",
			"Uppercase (A–Z)",
			"Digits (0–9)",
			"Symbols (!#$…)",
		]) {
			await user.click(within(panel).getByLabelText(name));
		}
		expect(within(panel).getByLabelText("Lowercase (a–z)")).toBeChecked();
		await waitFor(() =>
			expect(within(panel).getByTestId("generated-password").textContent).toMatch(
				/^[a-z]{20}$/,
			),
		);
		await user.click(screen.getByText("Use password"));
		await waitFor(() =>
			expect((screen.getByLabelText("Password") as HTMLInputElement).value).toMatch(
				/^[a-z]{20}$/,
			),
		);
	});
});

describe("keyboard", () => {
	it("Ctrl+F focuses search, Ctrl+N opens a new entry", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		await user.keyboard("{Control>}f{/Control}");
		expect(screen.getByRole("searchbox", { name: "Search entries" })).toHaveFocus();
		await user.keyboard("{Control>}n{/Control}");
		expect(await screen.findByRole("heading", { name: "New entry" })).toBeInTheDocument();
	});

	it("Ctrl+/ opens the shortcuts panel and Ctrl+K the palette", async () => {
		const { user } = await openVault();
		await user.keyboard("{Control>}/{/Control}");
		expect(
			await screen.findByRole("dialog", { name: "Keyboard shortcuts" }),
		).toBeInTheDocument();
		await user.keyboard("{Escape}");
		await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
		await user.keyboard("{Control>}k{/Control}");
		const palette = await screen.findByRole("dialog", { name: "Command palette" });
		await user.type(within(palette).getByRole("combobox"), "gitl");
		// Fuzzy matching may list weaker matches after it; GitLab ranks first.
		await waitFor(() => {
			const items = within(palette).getAllByRole("option");
			expect(items[0].textContent).toContain("GitLab");
			expect(items.some((i) => i.textContent?.includes("Bank"))).toBe(false);
		});
		await user.keyboard("{Enter}");
		expect(await screen.findByRole("heading", { name: "GitLab" })).toBeInTheDocument();
	});

	it("the copy event copies the password outside text fields only", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		await screen.findByRole("heading", { name: "GitHub" });
		const search = screen.getByRole("searchbox", { name: "Search entries" });
		search.focus();
		fireEvent.copy(search);
		await new Promise((r) => setTimeout(r, 0));
		expect(screen.queryByText(/copied · clears/)).not.toBeInTheDocument();
		option(/GitHub/).focus();
		fireEvent.copy(option(/GitHub/));
		expect(await screen.findByText("Password copied · clears in 20s")).toBeInTheDocument();
	});

	it("plain shortcuts leave selected text alone (M2-I08)", async () => {
		const { user } = await openVault();
		await user.click(option(/GitHub/));
		const heading = await screen.findByRole("heading", { name: "GitHub" });
		const range = document.createRange();
		range.selectNodeContents(heading);
		getSelection()?.removeAllRanges();
		getSelection()?.addRange(range);
		fireEvent.keyDown(document.body, { key: "Delete" });
		await new Promise((r) => setTimeout(r, 0));
		expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
		getSelection()?.removeAllRanges();
		fireEvent.keyDown(document.body, { key: "Delete" });
		expect(await screen.findByRole("alertdialog")).toBeInTheDocument();
	});

	it("arrow keys move through the list and Ctrl+L locks", async () => {
		const { user } = await openVault();
		await user.click(option(/Bank of Example/));
		await user.keyboard("{ArrowDown}");
		await waitFor(() =>
			expect(option(/Credit Union/)).toHaveAttribute("aria-selected", "true"),
		);
		expect(option(/Credit Union/)).toHaveFocus();
		await user.keyboard("{Control>}l{/Control}");
		expect(await screen.findByLabelText("Master password")).toBeInTheDocument();
	});
});

describe("settings", () => {
	it("clamps the clipboard timeout", async () => {
		const { user } = await openVault();
		await user.click(screen.getByRole("button", { name: /Settings/ }));
		const field = await screen.findByLabelText("Clear copied values after");
		await user.clear(field);
		await user.type(field, "2");
		await fireEvent.change(field);
		await waitFor(() => expect(field).toHaveValue(5));
	});
});

describe("database settings (milestone 3b)", () => {
	it("an AES-KDF database offers to switch to Argon2id instead of editing", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const realUnlock = vault.unlock.bind(vault);
		vault.unlock = async (path, password) => ({
			...(await realUnlock(path, password)),
			kdf: { kind: "aesKdf", rounds: 60000 },
		});
		const update = vi.spyOn(vault, "updateDatabase");
		const user = userEvent.setup();
		render(App, { apis: { vault, app: new MemoryAppApi() } });
		await unlock(user);
		await screen.findByRole("listbox", { name: "Entries" });
		await user.keyboard("{Control>}k{/Control}");
		await user.type(await screen.findByRole("combobox"), "Database Settings");
		await user.keyboard("{Enter}");
		await user.click(await screen.findByRole("button", { name: /Advanced: key derivation/ }));
		expect(screen.getByTestId("kdf-current")).toHaveTextContent("AES-KDF, 60000 rounds");
		expect(screen.queryByLabelText(/Memory/)).not.toBeInTheDocument();
		await user.click(screen.getByRole("button", { name: "Switch to Argon2id" }));
		await waitFor(() =>
			expect(update).toHaveBeenCalledWith({
				kdf: { memoryMib: 64, iterations: expect.any(Number), parallelism: 2 },
			}),
		);
	});

	it("the master password changes only with a matching confirmation", async () => {
		const { user, vault } = await openVault();
		const change = vi.spyOn(vault, "changeMasterPassword");
		await user.keyboard("{Control>}k{/Control}");
		await user.type(await screen.findByRole("combobox"), "Database Settings");
		await user.keyboard("{Enter}");
		await user.click(await screen.findByRole("button", { name: "Change master password…" }));
		const dialog = await screen.findByRole("dialog", { name: "Change master password" });
		await user.type(within(dialog).getByLabelText("New master password"), "correct horse");
		await user.type(
			within(dialog).getByLabelText("Confirm new master password"),
			"correct hors",
		);
		expect(within(dialog).getByRole("button", { name: "Change password" })).toBeDisabled();
		await user.type(within(dialog).getByLabelText("Confirm new master password"), "e");
		await user.click(within(dialog).getByRole("button", { name: "Change password" }));
		await waitFor(() => expect(change).toHaveBeenCalledWith("correct horse"));
	});
});

describe("master password fields (M3B-R03)", () => {
	it("are wiped when Lock closes the dialog", async () => {
		const { user } = await openVault();
		await user.keyboard("{Control>}k{/Control}");
		await user.type(await screen.findByRole("combobox"), "Database Settings");
		await user.keyboard("{Enter}");
		await user.click(await screen.findByRole("button", { name: "Change master password…" }));
		const dialog = await screen.findByRole("dialog", { name: "Change master password" });
		await user.type(within(dialog).getByLabelText("New master password"), "typed-then-locked");
		await user.keyboard("{Control>}l{/Control}");
		// The dialog's closing transition releases the page first.
		await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
		await waitFor(() => expect(document.body.style.pointerEvents).not.toBe("none"));
		await unlock(user);
		await screen.findByRole("listbox", { name: "Entries" });
		await user.keyboard("{Control>}k{/Control}");
		await user.type(await screen.findByRole("combobox"), "Database Settings");
		await user.keyboard("{Enter}");
		await user.click(await screen.findByRole("button", { name: "Change master password…" }));
		const again = await screen.findByRole("dialog", { name: "Change master password" });
		expect(within(again).getByLabelText("New master password")).toHaveValue("");
	}, 20_000);
});
