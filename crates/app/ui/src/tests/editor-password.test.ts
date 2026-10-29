import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";
import App from "$lib/App.svelte";
import { MemoryAppApi } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";

// The original bug (typed characters reverted by the field/draft sync) only
// shows in a real browser, where microtasks run between event listeners;
// jsdom cannot reproduce it. This guards the typed-and-saved path.
it("the password of a new entry can be typed and is saved", async () => {
	const user = userEvent.setup();
	const vault = new DemoVault({ delayMs: 0 });
	render(App, { apis: { vault, app: new MemoryAppApi() } });
	await user.type(await screen.findByLabelText("Master password"), "secret");
	await user.click(screen.getByRole("button", { name: "Unlock" }));
	await screen.findByRole("listbox", { name: /entries/i });
	await user.click(screen.getByRole("button", { name: "New entry" }));
	const field = await screen.findByLabelText("Password");
	await user.type(screen.getByLabelText("Title"), "Typed");
	await user.click(field);
	await user.keyboard("typed-by-hand");
	const now = screen.getByLabelText("Password");
	expect(now === field, "same element").toBe(true);
	expect(now).toHaveValue("typed-by-hand");
	expect(document.activeElement).toBe(now);
	await user.click(screen.getByRole("button", { name: "Save" }));
	await screen.findByRole("heading", { name: "Typed" });
	const saved = (await vault.listEntries()).find((e) => e.title === "Typed");
	expect(saved && (await vault.revealField(saved.id, "password"))).toBe("typed-by-hand");
});
