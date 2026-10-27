import { describe, expect, it } from "vitest";
import { MemoryAppApi, normalizeSettings, DEFAULT_SETTINGS } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";
import { AppState } from "$lib/state/app.svelte";

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((r) => (resolve = r));
	return { promise, resolve };
}

async function unlocked() {
	const vault = new DemoVault({ delayMs: 0 });
	const app = new AppState({ vault, app: new MemoryAppApi() });
	await app.init();
	await app.unlock("secret");
	expect(app.screen).toBe("vault");
	return { app, vault };
}

describe("unlock and lock", () => {
	it("rejects a wrong password and opens the vault with the right one", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		expect(app.screen).toBe("unlock");
		await app.unlock("wrong");
		expect(app.unlockError).toBe("wrongPassword");
		expect(app.screen).toBe("unlock");
		await app.unlock("secret");
		expect(app.screen).toBe("vault");
		expect(app.liveCount).toBe(9);
	});

	it("reports a recent file that no longer exists", async () => {
		const app = new AppState({ vault: new DemoVault({ delayMs: 0 }), app: new MemoryAppApi() });
		await app.init();
		app.selectRecent(2);
		await app.unlock("secret");
		expect(app.unlockError).toBe("notFound");
	});

	it("lock clears selection, revealed password, editor and toasts without asking", async () => {
		const { app } = await unlocked();
		await app.selectEntry("github");
		await app.toggleReveal();
		expect(app.revealed).toBe("tW8#qLz!v2Rm9@kP4sXe");
		await app.copy("password");
		await app.editEntry("github");
		app.updateDraft({ username: "changed" });
		app.lock();
		expect(app.confirm).toBeNull();
		expect(app.screen).toBe("unlock");
		expect(app.selectedId).toBeNull();
		expect(app.revealed).toBeNull();
		expect(app.editor).toBeNull();
		expect(app.toast).toBeNull();
		expect(app.entries).toEqual([]);
	});
});

describe("late asynchronous results are discarded", () => {
	it("a reveal that resolves after selecting another entry shows nothing", async () => {
		const { app, vault } = await unlocked();
		await app.selectEntry("github");
		const late = deferred<string>();
		vault.revealField = () => late.promise;
		const pending = app.toggleReveal();
		await app.selectEntry("gitlab");
		late.resolve("tW8#qLz!v2Rm9@kP4sXe");
		await pending;
		expect(app.selectedId).toBe("gitlab");
		expect(app.revealed).toBeNull();
	});

	it("a reveal that resolves after lock shows nothing", async () => {
		const { app, vault } = await unlocked();
		await app.selectEntry("github");
		const late = deferred<string>();
		vault.revealField = () => late.promise;
		const pending = app.toggleReveal();
		app.lock();
		late.resolve("tW8#qLz!v2Rm9@kP4sXe");
		await pending;
		expect(app.revealed).toBeNull();
	});

	it("an editor draft that resolves after lock is not recreated", async () => {
		const { app, vault } = await unlocked();
		await app.selectEntry("github");
		const late = deferred<Awaited<ReturnType<DemoVault["getEntryForEdit"]>>>();
		const real = await vault.getEntryForEdit("github");
		vault.getEntryForEdit = () => late.promise;
		const pending = app.editEntry("github");
		app.lock();
		late.resolve(real);
		await pending;
		expect(app.editor).toBeNull();
	});

	it("an unlock that resolves after switching database does not open the vault", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		const late = deferred<Awaited<ReturnType<DemoVault["unlock"]>>>();
		const realUnlock = vault.unlock.bind(vault);
		vault.unlock = () => late.promise;
		const pending = app.unlock("secret");
		app.selectRecent(1);
		late.resolve(await realUnlock("~/Documents/Personal.kdbx", "secret"));
		await pending;
		expect(app.screen).toBe("unlock");
		expect(app.db).toBeNull();
		expect(app.busy).toBeNull();
	});

	it("an entry detail that resolves after selecting another entry is ignored", async () => {
		const { app, vault } = await unlocked();
		const late = deferred<Awaited<ReturnType<DemoVault["getEntry"]>>>();
		const real = await vault.getEntry("github");
		const realGet = vault.getEntry.bind(vault);
		vault.getEntry = (id) => (id === "github" ? late.promise : realGet(id));
		const first = app.selectEntry("github");
		await app.selectEntry("gitlab");
		late.resolve(real);
		await first;
		expect(app.detail?.summary.id).toBe("gitlab");
	});
});

describe("leaving a modified editor", () => {
	async function dirtyEditor() {
		const ctx = await unlocked();
		await ctx.app.editEntry("github");
		ctx.app.updateDraft({ username: "someone-else" });
		expect(ctx.app.editorDirty).toBe(true);
		return ctx;
	}

	it("selecting another entry asks first; Keep editing stays", async () => {
		const { app } = await dirtyEditor();
		await app.selectEntry("gitlab");
		expect(app.confirm?.kind).toBe("leave");
		await app.resolveLeave("keep");
		expect(app.editor?.draft.username).toBe("someone-else");
		expect(app.selectedId).toBe("github");
	});

	it("Discard drops the draft and continues", async () => {
		const { app, vault } = await dirtyEditor();
		await app.selectEntry("gitlab");
		await app.resolveLeave("discard");
		expect(app.editor).toBeNull();
		expect(app.selectedId).toBe("gitlab");
		expect((await vault.getEntryForEdit("github")).username).toBe("alex.rivera");
	});

	it("Save stores the draft and continues", async () => {
		const { app, vault } = await dirtyEditor();
		await app.selectEntry("gitlab");
		await app.resolveLeave("save");
		expect(app.editor).toBeNull();
		expect(app.selectedId).toBe("gitlab");
		expect((await vault.getEntryForEdit("github")).username).toBe("someone-else");
	});

	it("New Entry, a group change and the native menu's Close Database are guarded too", async () => {
		const { app } = await dirtyEditor();
		await app.run("new_entry");
		expect(app.confirm?.kind).toBe("leave");
		await app.resolveLeave("keep");
		await app.selectGroup("email");
		expect(app.confirm?.kind).toBe("leave");
		await app.resolveLeave("keep");
		// Menu clicks arrive as command ids, exactly like this.
		await app.run("close_database");
		expect(app.confirm?.kind).toBe("leave");
		await app.resolveLeave("discard");
		expect(app.screen).toBe("unlock");
	});

	it("an unmodified editor is left without asking", async () => {
		const { app } = await unlocked();
		await app.editEntry("github");
		await app.selectEntry("gitlab");
		expect(app.confirm).toBeNull();
		expect(app.editor).toBeNull();
		expect(app.selectedId).toBe("gitlab");
	});
});

describe("entries, groups and saving", () => {
	it("delete moves to the recycle bin, restore brings it back", async () => {
		const { app } = await unlocked();
		await app.selectEntry("reddit");
		app.askDeleteEntry();
		expect(app.confirm).toMatchObject({ kind: "deleteEntry", permanent: false });
		await app.confirmDelete();
		expect(app.entries.find((e) => e.id === "reddit")?.groupId).toBe(app.recycleBinId);
		await app.selectGroup(app.recycleBinId as string);
		await app.selectEntry("reddit");
		await app.restoreSelected();
		expect(app.entries.find((e) => e.id === "reddit")?.groupId).toBe("social");
	});

	it("a failed save shows the banner and Retry clears it", async () => {
		const { app } = await unlocked();
		await app.run("demo_save_failure");
		expect(app.banner).toBe("saveFailed");
		expect(app.saveState).toBe("failed");
		await app.persist();
		expect(app.banner).toBeNull();
		expect(app.saveState).toBe("saved");
	});

	it("groups can be created, renamed and deleted", async () => {
		const { app } = await unlocked();
		await app.createGroup("Travel");
		const travel = app.groups.find((g) => g.name === "Travel");
		expect(travel).toBeDefined();
		await app.renameGroup(travel!.id, "Trips");
		expect(app.groups.some((g) => g.name === "Trips")).toBe(true);
		app.askDeleteGroup(travel!.id);
		await Promise.resolve();
		await app.confirmDelete();
		expect(app.groups.some((g) => g.name === "Trips")).toBe(false);
	});
});

describe("generator and settings", () => {
	it("keeps at least one character set and the length in range", async () => {
		const { app } = await unlocked();
		app.setGenerator({ lowercase: false, uppercase: false, digits: false, symbols: false });
		expect(app.generator.lowercase).toBe(true);
		app.setGenerator({ length: 500 });
		expect(app.generator.length).toBe(128);
		await app.regenerate();
		expect(app.generated).toMatch(/^[a-z]{128}$/);
	});

	it("clamps settings like the Rust core", async () => {
		const { app } = await unlocked();
		await app.updateSettings({ clipboard_clear_secs: 2, idle_lock_mins: 1000 });
		expect(app.settings.clipboard_clear_secs).toBe(5);
		expect(app.settings.idle_lock_mins).toBe(240);
		expect(
			normalizeSettings({
				...DEFAULT_SETTINGS,
				generator: { ...DEFAULT_SETTINGS.generator, length: 3 },
			}).generator.length,
		).toBe(8);
	});
});
