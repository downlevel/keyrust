import { describe, expect, it, vi } from "vitest";
import { MemoryAppApi, normalizeSettings, DEFAULT_SETTINGS } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";
import { AppState } from "$lib/state/app.svelte";
import type { Settings } from "$lib/api";

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

describe("inspection regressions", () => {
	async function dirty() {
		const ctx = await unlocked();
		await ctx.app.editEntry("github");
		ctx.app.updateDraft({ username: "someone-else" });
		return ctx;
	}

	it("a failed save stops the pending transition (M2-I01)", async () => {
		const { app, vault } = await dirty();
		await app.run("close_database");
		vault.failNextSave = true;
		await app.resolveLeave("save");
		expect(app.screen).toBe("vault");
		expect(app.banner).toBe("saveFailed");
	});

	it("Lock during Save cancels the pending transition (M2-I02)", async () => {
		const { app, vault } = await dirty();
		await app.selectEntry("gitlab");
		const late = deferred<void>();
		vault.save = () => late.promise;
		const pending = app.resolveLeave("save");
		await new Promise((r) => setTimeout(r, 0));
		app.lock();
		late.resolve();
		await pending;
		expect(app.screen).toBe("unlock");
		expect(app.selectedId).toBeNull();
	});

	it("a stale unlock never locks or replaces a newer session (M2-I03)", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		const realUnlock = vault.unlock.bind(vault);
		const late = deferred<void>();
		let calls = 0;
		vault.unlock = async (path, password) => {
			if (calls++ === 0) await late.promise;
			return realUnlock(path, password);
		};
		const first = app.unlock("secret");
		app.selectRecent(1);
		const second = app.unlock("secret");
		late.resolve();
		await Promise.all([first, second]);
		expect(app.screen).toBe("vault");
		expect(app.db?.name).toBe("Work");
		await expect(vault.listEntries()).resolves.not.toHaveLength(0);
	});

	it("Lock forgets the last generated password (M2-I04)", async () => {
		const { app } = await dirty();
		app.openGenerator();
		await vi.waitFor(() => expect(app.generated).not.toBe(""));
		app.lock();
		expect(app.generated).toBe("");
	});

	it("only the latest generator request is shown (M2-I05)", async () => {
		const { app, vault } = await unlocked();
		const answers: ((v: string) => void)[] = [];
		vault.generatePassword = () => new Promise((resolve) => answers.push(resolve));
		app.setGenerator({ length: 10 });
		app.setGenerator({ length: 30 });
		answers[1]("b".repeat(30));
		answers[0]("a".repeat(10));
		await new Promise((r) => setTimeout(r, 0));
		expect(app.generated).toBe("b".repeat(30));
	});

	it("a new default length applies to the generator (M2-I06)", async () => {
		const { app } = await unlocked();
		await app.updateSettings({ generator: { ...app.settings.generator, length: 32 } });
		expect(app.generator.length).toBe(32);
	});

	it("without the recycle bin, delete is permanent (M2-I07)", async () => {
		const { app } = await unlocked();
		await app.updateDatabase({ recycleBin: false });
		await app.selectEntry("reddit");
		app.askDeleteEntry();
		expect(app.confirm).toMatchObject({ kind: "deleteEntry", permanent: true });
		await app.confirmDelete();
		expect(app.entries.some((e) => e.id === "reddit")).toBe(false);
	});
});

describe("second inspection regressions", () => {
	it("one editor save at a time; no edits or navigation while it runs (M2-R201, M2-R202)", async () => {
		const { app, vault } = await unlocked();
		await app.newEntry();
		app.updateDraft({ title: "Netflix" });
		const late = deferred<string>();
		const realSave = vault.saveEntry.bind(vault);
		let saves = 0;
		vault.saveEntry = async (draft) => {
			saves++;
			await late.promise;
			return realSave(draft);
		};
		const first = app.saveEditor();
		expect(app.savingEntry).toBe(true);
		expect(await app.saveEditor()).toBe(false);
		app.updateDraft({ title: "Changed meanwhile" });
		expect(app.editor?.draft.title).toBe("Netflix");
		await app.newEntry();
		await app.selectEntry("gitlab");
		expect(app.editor?.draft.title).toBe("Netflix");
		late.resolve("");
		expect(await first).toBe(true);
		expect(saves).toBe(1);
		expect(app.entries.filter((e) => e.title === "Netflix")).toHaveLength(1);
		expect(app.savingEntry).toBe(false);
	});

	it("settings writes go out in order and the last answer wins (M2-R203)", async () => {
		const appApi = new MemoryAppApi();
		const app = new AppState({ vault: new DemoVault({ delayMs: 0 }), app: appApi });
		await app.init();
		const sent: Settings[] = [];
		const answers: (() => void)[] = [];
		const real = appApi.updateSettings.bind(appApi);
		appApi.updateSettings = (s) => {
			sent.push(s);
			return new Promise((resolve) => answers.push(() => resolve(real(s))));
		};
		const a = app.updateSettings({ clipboard_clear_secs: 30 });
		const b = app.updateSettings({ lock_on_minimize: true });
		await new Promise((r) => setTimeout(r, 0));
		expect(sent).toHaveLength(1);
		answers[0]();
		await a;
		expect(app.settings.lock_on_minimize).toBe(true);
		await new Promise((r) => setTimeout(r, 0));
		expect(sent).toHaveLength(2);
		expect(sent[1]).toMatchObject({ clipboard_clear_secs: 30, lock_on_minimize: true });
		answers[1]();
		await b;
		expect(app.settings).toMatchObject({ clipboard_clear_secs: 30, lock_on_minimize: true });
	});
});
