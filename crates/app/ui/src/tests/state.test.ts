import { describe, expect, it, vi } from "vitest";
import { MemoryAppApi, normalizeSettings, DEFAULT_SETTINGS } from "$lib/api/app";
import { DemoVault } from "$lib/api/demo";
import { VaultError } from "$lib/api/vault";
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
		// First into the recycle bin (whole), then permanently.
		expect(app.groups.find((g) => g.name === "Trips")?.inRecycleBin).toBe(true);
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
		const late = deferred<Awaited<ReturnType<DemoVault["save"]>>>();
		const info = await vault.databaseInfo();
		vault.save = () => late.promise;
		const pending = app.resolveLeave("save");
		await new Promise((r) => setTimeout(r, 0));
		app.lock();
		late.resolve(info);
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

describe("unsaved vault changes (milestone 3b)", () => {
	async function failingSave(kind: "externalChange" | "io") {
		const ctx = await unlocked();
		const exit = vi.spyOn(ctx.app.api.app, "exit").mockResolvedValue();
		if (kind === "externalChange") ctx.vault.externalChange = true;
		else ctx.vault.failNextSave = true;
		return { ...ctx, exit };
	}

	it("a failed save after an entry change guards Quit: cancel, discard", async () => {
		const { app, exit } = await failingSave("io");
		await app.selectEntry("github");
		app.askDeleteEntry();
		await app.confirmDelete();
		expect(app.vaultDirty).toBe(true);
		expect(app.banner).toBe("saveFailed");
		await app.run("quit");
		expect(app.confirm?.kind).toBe("unsaved");
		await app.resolveUnsaved("cancel");
		expect(exit).not.toHaveBeenCalled();
		await app.run("quit");
		await app.resolveUnsaved("discard");
		expect(exit).toHaveBeenCalledOnce();
	});

	it("Retry save continues only when the save succeeds", async () => {
		const { app, vault, exit } = await failingSave("externalChange");
		await app.createGroup("Travel");
		expect(app.banner).toBe("externalChange");
		await app.run("close_database");
		expect(app.confirm?.kind).toBe("unsaved");
		await app.resolveUnsaved("retry");
		expect(app.screen).toBe("vault");
		vault.externalChange = false;
		await app.run("close_database");
		await app.resolveUnsaved("retry");
		expect(app.screen).toBe("unlock");
		expect(exit).not.toHaveBeenCalled();
	});

	it("a failed save after a database setting guards opening another file", async () => {
		const { app } = await failingSave("io");
		await app.updateDatabase({ name: "Renamed" });
		await app.run("open_database");
		expect(app.confirm?.kind).toBe("unsaved");
	});

	it("leaving waits for a save in flight", async () => {
		const { app, vault } = await unlocked();
		const exit = vi.spyOn(app.api.app, "exit").mockResolvedValue();
		const late = deferred<Awaited<ReturnType<DemoVault["save"]>>>();
		const info = await vault.databaseInfo();
		vault.save = () => late.promise;
		const saving = app.persist();
		const quitting = app.run("quit");
		await new Promise((r) => setTimeout(r, 0));
		expect(exit).not.toHaveBeenCalled();
		late.resolve({ ...info, dirty: false });
		await saving;
		await quitting;
		expect(exit).toHaveBeenCalledOnce();
	});

	it("Keep mine overwrites, Reload drops unsaved changes", async () => {
		const { app, vault } = await failingSave("externalChange");
		await app.createGroup("Mine");
		expect(app.banner).toBe("externalChange");
		await app.keepMine();
		expect(app.banner).toBeNull();
		expect(app.vaultDirty).toBe(false);

		vault.externalChange = true;
		await app.createGroup("Lost");
		await app.reloadExternal();
		expect(app.banner).toBeNull();
		expect(app.vaultDirty).toBe(false);
		expect(app.groups.some((g) => g.name === "Lost")).toBe(false);
	});

	it("coming back to the window shows a change made by another app", async () => {
		const { app, vault } = await unlocked();
		await app.checkExternal();
		expect(app.banner).toBeNull();
		vault.externalChange = true;
		await app.checkExternal();
		expect(app.banner).toBe("externalChange");
	});

	it("Lock never asks, even with unsaved changes", async () => {
		const { app } = await failingSave("io");
		await app.createGroup("Unsaved");
		app.lock();
		expect(app.confirm).toBeNull();
		expect(app.vaultDirty).toBe(false);
		expect(app.screen).toBe("unlock");
	});

	it("a new master password is saved straight away", async () => {
		const { app, vault } = await unlocked();
		const change = vi.spyOn(vault, "changeMasterPassword");
		const save = vi.spyOn(vault, "save");
		expect(await app.changeMasterPassword("brand-new")).toBe("changed");
		expect(change).toHaveBeenCalledWith("brand-new");
		expect(save).toHaveBeenCalled();
		expect(app.vaultDirty).toBe(false);
	});
});

describe("3b inspection regressions", () => {
	it("Quit waits for a change still in flight and sees its failed save (M3B-01)", async () => {
		const { app, vault } = await unlocked();
		const exit = vi.spyOn(app.api.app, "exit").mockResolvedValue();
		const late = deferred<void>();
		const realCreate = vault.createGroup.bind(vault);
		vault.createGroup = async (parent, name) => {
			await late.promise;
			return realCreate(parent, name);
		};
		vault.failNextSave = true;
		const creating = app.createGroup("Travel");
		const quitting = app.run("quit");
		await new Promise((r) => setTimeout(r, 0));
		expect(exit).not.toHaveBeenCalled();
		late.resolve();
		await creating;
		await quitting;
		expect(exit).not.toHaveBeenCalled();
		expect(app.confirm?.kind).toBe("unsaved");
	});

	it("a failed refresh does not skip the save (M3B-01)", async () => {
		const { app, vault } = await unlocked();
		const save = vi.spyOn(vault, "save");
		const realTree = vault.tree.bind(vault);
		let failed = false;
		vault.tree = async () => {
			if (!failed) {
				failed = true;
				throw new VaultError("io");
			}
			return realTree();
		};
		await app.createGroup("Travel");
		expect(save).toHaveBeenCalled();
		expect(app.vaultDirty).toBe(false);
	});

	it("the backend decides whether anything is unsaved when leaving (M3B-01)", async () => {
		const { app, vault } = await unlocked();
		const exit = vi.spyOn(app.api.app, "exit").mockResolvedValue();
		app.vaultDirty = false;
		await vault.updateDatabase({ name: "Changed behind the UI's back" });
		await app.run("quit");
		expect(app.confirm?.kind).toBe("unsaved");
		expect(exit).not.toHaveBeenCalled();
	});

	it("password change outcomes are reported as they are (M3B-03)", async () => {
		const { app, vault } = await unlocked();
		expect(await app.changeMasterPassword("one")).toBe("changed");

		vault.failNextSave = true;
		expect(await app.changeMasterPassword("two")).toBe("pending");
		const reload = vi.spyOn(vault, "reload");
		await app.discardUnsaved();
		expect(reload).toHaveBeenCalled();
		expect(app.vaultDirty).toBe(false);

		vault.save = () => Promise.reject(new VaultError("notDurable"));
		expect(await app.changeMasterPassword("three")).toBe("notDurable");

		vault.changeMasterPassword = () => Promise.reject(new VaultError("invalid"));
		expect(await app.changeMasterPassword("four")).toBe("rejected");
	});
});

describe("3b second inspection regressions", () => {
	it("a stale Retry save never closes the next database (M3B-R01)", async () => {
		const { app, vault } = await unlocked();
		const exit = vi.spyOn(app.api.app, "exit").mockResolvedValue();
		vault.failNextSave = true;
		await app.createGroup("Unsaved");
		await app.run("quit");
		expect(app.confirm?.kind).toBe("unsaved");
		const late = deferred<Awaited<ReturnType<DemoVault["save"]>>>();
		const info = await vault.databaseInfo();
		vault.save = () => late.promise;
		const retry = app.resolveUnsaved("retry");
		await new Promise((r) => setTimeout(r, 0));
		app.lock();
		await app.unlock("secret");
		late.resolve({ ...info, dirty: false });
		await retry;
		expect(exit).not.toHaveBeenCalled();
		expect(app.screen).toBe("vault");
	});

	it("entries of a deleted group count as recycled (M3B-R02)", async () => {
		const { app } = await unlocked();
		const before = app.liveCount;
		const banking = app.groups.find((g) => g.name === "Banking");
		app.askDeleteGroup(banking!.id);
		await Promise.resolve();
		await app.confirmDelete();
		expect(app.liveCount).toBe(before - 2);
		expect(app.groups.find((g) => g.name === "Banking")?.inRecycleBin).toBe(true);
		await app.selectGroup(app.recycleBinId!);
		expect(app.visibleEntries.map((e) => e.title)).toEqual(
			expect.arrayContaining(["Bank of Example", "Credit Union"]),
		);
		await app.selectEntry("bank");
		expect(app.inRecycleBin).toBe(true);
		await app.editEntry("bank");
		expect(app.editor).toBeNull();
		app.askDeleteEntry("bank");
		expect(app.confirm).toMatchObject({ kind: "deleteEntry", permanent: true });
	});
});

describe("3b second inspection regressions (guard waiting)", () => {
	it("an exit waiting for a save is dropped if Lock happens meanwhile (M3B-R01)", async () => {
		const { app, vault } = await unlocked();
		const exit = vi.spyOn(app.api.app, "exit").mockResolvedValue();
		const late = deferred<Awaited<ReturnType<DemoVault["save"]>>>();
		const info = await vault.databaseInfo();
		vault.save = () => late.promise;
		void app.persist();
		const quitting = app.run("quit");
		await new Promise((r) => setTimeout(r, 0));
		app.lock();
		await app.unlock("secret");
		late.resolve({ ...info, dirty: false });
		await quitting;
		expect(exit).not.toHaveBeenCalled();
		expect(app.screen).toBe("vault");
	});
});

describe("automatic lock", () => {
	/** The demo vault posing as the real one, for what the demo never does. */
	function asReal(vault: DemoVault) {
		return Object.defineProperty(vault, "isDemo", { value: false });
	}

	it("clears the session like Lock, without calling the backend or asking", async () => {
		const { app, vault } = await unlocked();
		const lock = vi.spyOn(vault, "lock");
		await app.selectEntry("github");
		await app.toggleReveal();
		await app.editEntry("github");
		app.updateDraft({ username: "changed" });
		app.autoLocked("idle");
		expect(lock).not.toHaveBeenCalled();
		expect(app.confirm).toBeNull();
		expect(app.screen).toBe("unlock");
		expect(app.selectedId).toBeNull();
		expect(app.revealed).toBeNull();
		expect(app.editor).toBeNull();
		expect(app.entries).toEqual([]);
		expect(app.lockNotice).toEqual({ reason: "idle", minutes: 5, lostChanges: true });
	});

	it("records the reason and no lost changes when everything was saved", async () => {
		const { app } = await unlocked();
		app.autoLocked("screenLock");
		expect(app.lockNotice).toEqual({ reason: "screenLock", minutes: 5, lostChanges: false });
	});

	it("ignores a late event about an older backend session", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const real = vault.unlock.bind(vault);
		// The backend installed this session at generation 5.
		vault.unlock = async (path, password) => ({ ...(await real(path, password)), session: 5 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		await app.unlock("secret");
		// A lock that started generation 5 locked the previous session.
		app.autoLocked("idle", 5);
		expect(app.screen).toBe("vault");
		expect(app.lockNotice).toBeNull();
		// One that started generation 6 locked this one.
		app.autoLocked("idle", 6);
		expect(app.screen).toBe("unlock");
		expect(app.lockNotice?.reason).toBe("idle");
	});

	it("ignores a late event while the new session is still loading", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const real = vault.unlock.bind(vault);
		vault.unlock = async (path, password) => ({ ...(await real(path, password)), session: 5 });
		const lock = vi.spyOn(vault, "lock");
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		const tree = deferred<Awaited<ReturnType<DemoVault["tree"]>>>();
		const realTree = vault.tree.bind(vault);
		vault.tree = () => tree.promise;
		const pending = app.unlock("secret");
		await vi.waitFor(() => expect(app.busy).toBe("unlock"));
		// Let the unlock answer; the tree is still loading.
		await new Promise((r) => setTimeout(r, 0));
		app.autoLocked("idle", 5);
		tree.resolve(await realTree());
		await pending;
		expect(app.screen).toBe("vault");
		expect(app.lockNotice).toBeNull();
		expect(lock).not.toHaveBeenCalled();
	});

	it("reconciles an event received while the unlock answer is pending", async () => {
		const setup = async () => {
			const vault = new DemoVault({ delayMs: 0 });
			const real = vault.unlock.bind(vault);
			const answer = deferred<void>();
			// Installed at generation 5, answered only when the test says so.
			vault.unlock = async (path, password) => {
				const info = await real(path, password);
				await answer.promise;
				return { ...info, session: 5 };
			};
			const app = new AppState({ vault, app: new MemoryAppApi() });
			await app.init();
			const pending = app.unlock("secret");
			await vi.waitFor(() => expect(app.busy).toBe("unlock"));
			return { app, answer, pending };
		};
		// About an older session (it started generation 5): the unlock goes on.
		const older = await setup();
		older.app.autoLocked("screenLock", 5);
		older.answer.resolve();
		await older.pending;
		expect(older.app.screen).toBe("vault");
		expect(older.app.lockNotice).toBeNull();
		// Started after the install (generation 6): this session is closed.
		const newer = await setup();
		newer.app.autoLocked("screenLock", 6);
		newer.answer.resolve();
		await newer.pending;
		expect(newer.app.screen).toBe("unlock");
		expect(newer.app.db).toBeNull();
		expect(newer.app.lockNotice?.reason).toBe("interrupted");
	});

	it("is ignored when nothing is open or being opened", async () => {
		const app = new AppState({ vault: new DemoVault({ delayMs: 0 }), app: new MemoryAppApi() });
		await app.init();
		app.autoLocked("suspend");
		expect(app.lockNotice).toBeNull();
		expect(app.screen).toBe("unlock");
	});

	it("abandons an unlock in progress", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		const late = deferred<Awaited<ReturnType<DemoVault["unlock"]>>>();
		const real = vault.unlock.bind(vault);
		vault.unlock = () => late.promise;
		const pending = app.unlock("secret");
		app.autoLocked("screenLock");
		expect(app.busy).toBeNull();
		expect(app.lockNotice?.reason).toBe("interrupted");
		late.resolve(await real(app.recents[0].path, "secret"));
		await pending;
		expect(app.screen).toBe("unlock");
		expect(app.db).toBeNull();
	});

	it("explains an unlock the backend discarded because of a lock", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		vault.unlock = () => Promise.reject(new VaultError("locked"));
		await app.unlock("secret");
		expect(app.unlockError).toBeNull();
		expect(app.lockNotice?.reason).toBe("interrupted");
		app.selectRecent(1);
		expect(app.lockNotice).toBeNull();
	});

	it("a database beyond the open limits gets its own message", async () => {
		const vault = new DemoVault({ delayMs: 0 });
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		vault.unlock = () => Promise.reject(new VaultError("tooExpensive"));
		await app.unlock("secret");
		expect(app.unlockError).toBe("tooExpensive");
	});

	it("reports activity at most every 15 seconds, only with a vault open", async () => {
		const vault = asReal(new DemoVault({ delayMs: 0 }));
		const activity = vi.spyOn(vault, "activity");
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		app.reportActivity(1_000);
		expect(activity).not.toHaveBeenCalled();
		await app.unlock("secret");
		app.reportActivity(1_000);
		app.reportActivity(15_999);
		expect(activity).toHaveBeenCalledTimes(1);
		app.reportActivity(16_000);
		expect(activity).toHaveBeenCalledTimes(2);
		app.lock();
		app.reportActivity(40_000);
		expect(activity).toHaveBeenCalledTimes(2);
	});

	it("uses a monotonic clock: setting the system clock back does not silence activity", async () => {
		const vault = asReal(new DemoVault({ delayMs: 0 }));
		const activity = vi.spyOn(vault, "activity");
		const app = new AppState({ vault, app: new MemoryAppApi() });
		await app.init();
		await app.unlock("secret");
		const now = vi.spyOn(performance, "now").mockReturnValue(100_000);
		const wall = vi.spyOn(Date, "now").mockReturnValue(10_000_000);
		app.reportActivity();
		wall.mockReturnValue(10_000_000 - 3_600_000); // an hour back
		now.mockReturnValue(116_000);
		app.reportActivity();
		expect(activity).toHaveBeenCalledTimes(2);
		now.mockRestore();
		wall.mockRestore();
	});

	it("never reports activity in the demo", async () => {
		const { app, vault } = await unlocked();
		const activity = vi.spyOn(vault, "activity");
		app.reportActivity(1_000);
		app.reportActivity(100_000);
		expect(activity).not.toHaveBeenCalled();
	});
});

describe("KDF limits in the forms", () => {
	it("brings typed-in parameters within the limits Keyrust opens", async () => {
		const { clampKdf } = await import("$lib/api/app");
		expect(clampKdf({ memoryMib: 5000, iterations: 0, parallelism: 300 })).toEqual({
			memoryMib: 2048,
			iterations: 1,
			parallelism: 256,
		});
		expect(clampKdf({ memoryMib: 64, iterations: 3, parallelism: 2 })).toEqual({
			memoryMib: 64,
			iterations: 3,
			parallelism: 2,
		});
	});
});
