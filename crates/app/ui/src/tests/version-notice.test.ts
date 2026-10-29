import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "$lib/App.svelte";
import { MemoryAppApi } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";
import type { DatabaseInfo } from "$lib/api";
import { AppState } from "$lib/state/app.svelte";

/**
 * The demo with a file of the given KDBX version: like the Rust core, a save
 * that replaced the file reports 4.1 and a refused save keeps the old version.
 */
class VersionedVault extends DemoVault {
	constructor(private version: [number, number]) {
		super({ delayMs: 0 });
	}

	private copy(): [number, number] {
		return [this.version[0], this.version[1]];
	}

	override async unlock(path: string, password: string): Promise<DatabaseInfo> {
		return { ...(await super.unlock(path, password)), version: this.copy() };
	}

	override async databaseInfo(): Promise<DatabaseInfo> {
		return { ...(await super.databaseInfo()), version: this.copy() };
	}

	override async updateDatabase(patch: Parameters<DemoVault["updateDatabase"]>[0]) {
		return { ...(await super.updateDatabase(patch)), version: this.copy() };
	}

	override async save(): Promise<DatabaseInfo> {
		const info = await super.save();
		this.version = [4, 1];
		return { ...info, version: [4, 1] };
	}
}

async function unlocked(version: [number, number]) {
	const vault = new VersionedVault(version);
	const app = new AppState({ vault, app: new MemoryAppApi() });
	await app.init();
	await app.unlock("secret");
	expect(app.screen).toBe("vault");
	return { app, vault };
}

describe("KDBX conversion notice", () => {
	it("is shown for KDBX 3.1 and 4.0, not for 4.1", async () => {
		expect((await unlocked([3, 1])).app.versionNotice).toBe("3.1");
		expect((await unlocked([4, 0])).app.versionNotice).toBe("4.0");
		expect((await unlocked([4, 1])).app.versionNotice).toBeNull();
	});

	it("the demo database is KDBX 4.1: no notice", async () => {
		const app = new AppState({ vault: new DemoVault({ delayMs: 0 }), app: new MemoryAppApi() });
		await app.init();
		await app.unlock("secret");
		expect(app.versionNotice).toBeNull();
	});

	it("stays after a failed save and goes after a successful one", async () => {
		const { app, vault } = await unlocked([3, 1]);
		vault.failNextSave = true;
		expect(await app.persist()).toBe(false);
		expect(app.banner).toBe("saveFailed");
		expect(app.versionNotice).toBe("3.1");
		expect(await app.persist()).toBe(true);
		expect(app.versionNotice).toBeNull();
	});

	it("stays after a save refused for an external change", async () => {
		const { app, vault } = await unlocked([4, 0]);
		vault.externalChange = true;
		expect(await app.persist()).toBe(false);
		expect(app.banner).toBe("externalChange");
		expect(app.versionNotice).toBe("4.0");
	});

	it("Dismiss hides it until the next unlock of a file that was not saved", async () => {
		const { app } = await unlocked([3, 1]);
		app.versionNoticeDismissed = true;
		expect(app.versionNotice).toBeNull();
		app.lock();
		await app.unlock("secret");
		expect(app.versionNotice).toBe("3.1");
	});

	it("is rendered as a status next to the other banners, never instead of them", async () => {
		const vault = new VersionedVault([3, 1]);
		const user = userEvent.setup();
		render(App, { apis: { vault, app: new MemoryAppApi() } });
		await user.type(await screen.findByLabelText("Master password"), "secret");
		await user.click(screen.getByRole("button", { name: "Unlock" }));
		const notice = await screen.findByText(/This database uses KDBX 3\.1\./);
		expect(notice.closest('[role="status"]')).not.toBeNull();

		vault.failNextSave = true;
		await user.keyboard("{Control>}s{/Control}");
		expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t save the database.");
		expect(screen.getByText(/This database uses KDBX 3\.1\./)).toBeInTheDocument();

		await user.click(screen.getByRole("button", { name: "Dismiss" }));
		expect(screen.queryByText(/This database uses KDBX/)).toBeNull();
		expect(screen.getByRole("alert")).toBeInTheDocument();
	});
});
