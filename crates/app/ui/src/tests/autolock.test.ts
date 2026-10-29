import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

// Inside Tauri: App subscribes to the backend's `locked` event.
const handlers = new Map<string, (event: { payload: unknown }) => void>();
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
	listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
		handlers.set(name, handler);
		return () => handlers.delete(name);
	},
}));

const { default: App } = await import("$lib/App.svelte");
const { MemoryAppApi } = await import("$lib/api/app");
const { DemoVault } = await import("$lib/api/demo");

async function openVault() {
	const user = userEvent.setup();
	render(App, { apis: { vault: new DemoVault({ delayMs: 0 }), app: new MemoryAppApi() } });
	await user.type(await screen.findByLabelText("Master password"), "secret");
	await user.click(screen.getByRole("button", { name: "Unlock" }));
	await screen.findByRole("listbox", { name: /entries/i });
	await vi.waitFor(() => expect(handlers.has("locked")).toBe(true));
}

describe("the locked event", () => {
	it.each([
		["idle", "Locked after 5 minutes of inactivity."],
		["screenLock", "Locked because the screen was locked."],
		["suspend", "Locked when the computer went to sleep."],
		["minimize", "Locked when the window was minimized."],
	])("%s: back to Unlock with the reason", async (reason, text) => {
		await openVault();
		handlers.get("locked")?.({ payload: { reason } });
		expect(await screen.findByTestId("lock-notice")).toHaveTextContent(text);
		expect(screen.queryByRole("listbox", { name: /entries/i })).toBeNull();
		expect(screen.getByLabelText("Master password")).toHaveValue("");
	});
});
