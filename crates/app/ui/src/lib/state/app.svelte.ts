import { DEFAULT_SETTINGS } from "$lib/api/app";
import {
	VaultError,
	type VaultErrorKind,
	type Apis,
	type AppInfo,
	type Cipher,
	type DatabaseInfo,
	type EntryDetail,
	type EntryDraft,
	type EntryId,
	type EntrySummary,
	type Field,
	type GeneratorOptions,
	type GroupId,
	type GroupNode,
	type Argon2idParams,
	type DatabasePatch,
	type RecentFile,
	type Settings,
} from "$lib/api";
import {
	command,
	commandForKeydown,
	isEditableTarget,
	type CommandId,
} from "$lib/commands/commands";

export type Screen = "welcome" | "unlock" | "create" | "vault";
export type Overlay = "settings" | "shortcuts" | "palette" | "generator" | "masterPassword" | null;
export type SettingsTab = "general" | "database" | "about";
export type LeaveChoice = "save" | "discard" | "keep";

/** "All entries": every group except the recycle bin. */
export const ALL = "all";

export interface EditorState {
	draft: EntryDraft;
	original: EntryDraft;
	isNew: boolean;
}

export type Confirm =
	| { kind: "deleteEntry"; id: EntryId; title: string; permanent: boolean }
	| { kind: "deleteGroup"; id: GroupId; name: string }
	| { kind: "leave"; action: () => void | Promise<void> }
	/** The vault has changes the last save could not write. */
	| { kind: "unsaved"; action: () => void | Promise<void> };

export type UnlockError = "wrongPassword" | "notFound" | "invalidFile" | "failed";

/** Rejections the user caused (a cancelled dialog) are not errors. */
function cancelled(e: unknown): boolean {
	return e instanceof VaultError && e.kind === "cancelled";
}

export interface FlatGroup {
	id: GroupId;
	name: string;
	depth: number;
	count: number;
	isRecycleBin: boolean;
}

/**
 * Application state and actions. Every asynchronous result is applied only if
 * the session (unlock / lock / create / database change) and, where relevant,
 * the selection (entry change, editor open / close) are still the ones that
 * started the call: a late answer can never bring back a secret or a draft.
 */
export class AppState {
	screen = $state<Screen>("unlock");
	recents = $state<RecentFile[]>([]);
	recentIndex = $state(0);
	unlockError = $state<UnlockError | null>(null);
	createError = $state(false);
	busy = $state<"unlock" | "create" | "save" | "benchmark" | null>(null);

	db = $state<DatabaseInfo | null>(null);
	tree = $state<GroupNode | null>(null);
	entries = $state<EntrySummary[]>([]);
	groupId = $state<GroupId>(ALL);
	query = $state("");
	selectedId = $state<EntryId | null>(null);
	detail = $state<EntryDetail | null>(null);
	revealed = $state<string | null>(null);
	editor = $state<EditorState | null>(null);
	/** An editor save is in flight: the form and navigation wait for it. */
	savingEntry = $state(false);

	toast = $state<{ field: Field; secs: number } | null>(null);
	banner = $state<"externalChange" | "saveFailed" | null>(null);
	saveState = $state<"saved" | "saving" | "failed">("saved");
	/**
	 * The vault has changes that are not in the file: set by every change,
	 * cleared only by a save that reached the disk. Independent of the editor.
	 */
	vaultDirty = $state(false);
	/** The save in flight, if any: leaving waits for it. */
	private saving: Promise<boolean> | null = null;
	/** Vault changes in flight (mutation, refresh, save): leaving waits for them. */
	private pending = new Set<Promise<unknown>>();
	/** Why the last save failed, if it did. */
	lastSaveError: VaultErrorKind | null = null;

	overlay = $state<Overlay>(null);
	settingsTab = $state<SettingsTab>("general");
	confirm = $state<Confirm | null>(null);
	renamingGroupId = $state<GroupId | null>(null);
	sidebarCollapsed = $state(false);
	/** Incremented to ask the search input to take focus. */
	focusSearch = $state(0);

	settings = $state<Settings>(structuredClone(DEFAULT_SETTINGS));
	appInfo = $state<AppInfo | null>(null);

	generator = $state<GeneratorOptions>(generatorFromSettings(DEFAULT_SETTINGS));
	generated = $state("");

	private session = 0;
	private selection = 0;
	/** Latest generator request; older answers are dropped. */
	private generatorRequest = 0;
	/**
	 * Session-changing backend calls (unlock, create, lock) run one at a time,
	 * so a stale completion can be undone before the next one starts and can
	 * never replace or lock a newer session.
	 */
	private sessionOps: Promise<void> = Promise.resolve();
	/** Settings writes go out one at a time; only the latest answer is applied. */
	private settingsQueue: Promise<void> = Promise.resolve();
	private settingsWrite = 0;
	private toastTimer: ReturnType<typeof setInterval> | null = null;

	readonly isDemo: boolean;

	constructor(readonly api: Apis) {
		this.isDemo = api.vault.isDemo;
	}

	// ---------------------------------------------------------------- derived

	readonly groups: FlatGroup[] = $derived.by(() => {
		const out: FlatGroup[] = [];
		const walk = (node: GroupNode, depth: number) => {
			for (const child of node.children) {
				out.push({
					id: child.id,
					name: child.name,
					depth,
					count: this.entries.filter((e) => e.groupId === child.id).length,
					isRecycleBin: child.isRecycleBin,
				});
				walk(child, depth + 1);
			}
		};
		if (this.tree) walk(this.tree, 0);
		return out;
	});

	/** Ids come from the backend: the root and recycle bin are found by role. */
	readonly rootId = $derived(this.tree?.id ?? "");
	readonly recycleBinId = $derived(this.tree?.children.find((c) => c.isRecycleBin)?.id ?? null);

	readonly liveCount = $derived(
		this.entries.filter((e) => e.groupId !== this.recycleBinId).length,
	);

	readonly visibleEntries: EntrySummary[] = $derived.by(() => {
		const q = this.query.trim().toLowerCase();
		return this.entries
			.filter((e) =>
				this.groupId === ALL ? e.groupId !== this.recycleBinId : e.groupId === this.groupId,
			)
			.filter((e) => !q || `${e.title} ${e.username} ${e.url}`.toLowerCase().includes(q))
			.sort((a, b) => a.title.localeCompare(b.title));
	});

	readonly selected: EntrySummary | null = $derived(
		this.entries.find((e) => e.id === this.selectedId) ?? null,
	);

	readonly editorDirty = $derived(
		!!this.editor && JSON.stringify(this.editor.draft) !== JSON.stringify(this.editor.original),
	);

	readonly inRecycleBin = $derived(this.selected?.groupId === this.recycleBinId);

	// ---------------------------------------------------------------- start

	async init() {
		const [settings, info] = await Promise.all([
			this.api.app.getSettings().catch(() => structuredClone(DEFAULT_SETTINGS)),
			this.api.app.appInfo().catch(() => null),
		]);
		this.settings = settings;
		this.appInfo = info;
		this.generator = generatorFromSettings(settings);
		await this.loadRecents();
	}

	private async loadRecents() {
		this.recents = await this.api.vault.recentFiles();
		this.recentIndex = 0;
		this.screen = this.recents.length ? "unlock" : "welcome";
	}

	selectRecent(index: number) {
		if (index === this.recentIndex) return;
		this.session++; // another database: any pending unlock is stale
		this.recentIndex = index;
		this.unlockError = null;
		this.busy = null;
	}

	async openOtherFile() {
		let file: RecentFile;
		try {
			file = await this.api.vault.chooseFile();
		} catch (e) {
			if (!cancelled(e)) this.unlockError = "failed";
			return;
		}
		this.session++;
		const existing = this.recents.findIndex((r) => r.path === file.path);
		if (existing >= 0) this.recentIndex = existing;
		else {
			this.recents = [file, ...this.recents];
			this.recentIndex = 0;
		}
		this.unlockError = null;
		this.busy = null;
		this.screen = "unlock";
	}

	private serialize<T>(op: () => Promise<T>): Promise<T> {
		const run = this.sessionOps.then(op, op);
		this.sessionOps = run.then(
			() => {},
			() => {},
		);
		return run;
	}

	/** Opens a session in the backend unless the UI moved on meanwhile. */
	private openSession(session: number, open: () => Promise<DatabaseInfo>) {
		return this.serialize(async () => {
			const info = await open();
			if (session === this.session) return info;
			await this.api.vault.lock();
			return null;
		});
	}

	async unlock(password: string) {
		const recent = this.recents[this.recentIndex];
		if (!recent || this.busy) return;
		const session = ++this.session;
		this.busy = "unlock";
		this.unlockError = null;
		try {
			const info = await this.openSession(session, () =>
				this.api.vault.unlock(recent.path, password),
			);
			if (!info || session !== this.session) return;
			await this.enterVault(info, session);
		} catch (e) {
			if (session !== this.session) return;
			const kind = e instanceof VaultError ? e.kind : "io";
			this.unlockError =
				kind === "wrongKey"
					? "wrongPassword"
					: kind === "notFound"
						? "notFound"
						: kind === "invalidFormat" || kind === "unsupportedVersion"
							? "invalidFile"
							: "failed";
		} finally {
			if (session === this.session) this.busy = null;
		}
	}

	startCreate() {
		this.session++;
		this.createError = false;
		this.busy = null;
		this.screen = "create";
	}

	cancelCreate() {
		this.session++;
		this.busy = null;
		this.screen = this.recents.length ? "unlock" : "welcome";
	}

	async create(options: {
		path: string;
		name: string;
		password: string;
		cipher: Cipher;
		kdf: Argon2idParams;
	}) {
		if (this.busy) return;
		const session = ++this.session;
		this.busy = "create";
		this.createError = false;
		try {
			const info = await this.openSession(session, () => this.api.vault.create(options));
			if (!info || session !== this.session) return;
			this.recents = [
				{ name: info.name, path: options.path, missing: false },
				...this.recents.filter((r) => r.path !== options.path),
			];
			this.recentIndex = 0;
			await this.enterVault(info, session);
		} catch {
			if (session === this.session) this.createError = true;
		} finally {
			if (session === this.session) this.busy = null;
		}
	}

	private async enterVault(info: DatabaseInfo, session: number) {
		const [tree, entries] = await Promise.all([
			this.api.vault.tree(),
			this.api.vault.listEntries(),
		]);
		if (session !== this.session) return;
		this.db = info;
		this.vaultDirty = info.dirty;
		this.tree = tree;
		this.entries = entries;
		this.groupId = ALL;
		this.query = "";
		this.selectedId = null;
		this.detail = null;
		this.revealed = null;
		this.editor = null;
		this.banner = null;
		this.saveState = "saved";
		this.screen = "vault";
	}

	/** Immediate: discards the editor without asking. */
	lock() {
		this.session++;
		this.selection++;
		this.db = null;
		this.tree = null;
		this.entries = [];
		this.selectedId = null;
		this.detail = null;
		this.revealed = null;
		this.editor = null;
		this.toast = null;
		this.stopToastTimer();
		this.banner = null;
		this.confirm = null;
		this.overlay = null;
		this.renamingGroupId = null;
		this.query = "";
		this.busy = null;
		this.unlockError = null;
		this.generated = "";
		this.vaultDirty = false;
		this.saveState = "saved";
		void this.serialize(() => this.api.vault.lock());
		this.screen = this.recents.length ? "unlock" : "welcome";
	}

	// ---------------------------------------------------------------- leaving the editor

	/** Runs `action` once a modified editor has been saved or discarded. */
	async requestLeave(action: () => void | Promise<void>) {
		// Nothing replaces the editor while its save is in flight (Lock still does).
		if (this.savingEntry) return;
		if (this.editor && this.editorDirty) {
			this.confirm = { kind: "leave", action };
			return;
		}
		this.closeEditor();
		await action();
	}

	async resolveLeave(choice: LeaveChoice) {
		const pending = this.confirm;
		if (pending?.kind !== "leave") return;
		this.confirm = null;
		if (choice === "keep") return;
		const session = this.session;
		// The transition runs only after a save that reached the file, and only
		// if nothing (Lock, another database) replaced the session meanwhile.
		if (choice === "save" && !(await this.saveEditor())) return;
		if (session !== this.session) return;
		this.closeEditor();
		await pending.action();
	}

	private closeEditor() {
		if (!this.editor) return;
		this.selection++;
		this.editor = null;
		this.generated = "";
		if (this.overlay === "generator") this.overlay = null;
	}

	// ---------------------------------------------------------------- navigation

	selectGroup(id: GroupId) {
		return this.requestLeave(() => {
			this.groupId = id;
			if (this.selected && !this.visibleEntries.some((e) => e.id === this.selectedId)) {
				this.clearSelection();
			}
		});
	}

	selectEntry(id: EntryId) {
		return this.requestLeave(() => this.loadEntry(id));
	}

	private clearSelection() {
		this.selection++;
		this.selectedId = null;
		this.detail = null;
		this.revealed = null;
	}

	private async loadEntry(id: EntryId) {
		const selection = ++this.selection;
		const session = this.session;
		this.selectedId = id;
		this.detail = null;
		this.revealed = null;
		const detail = await this.api.vault.getEntry(id).catch(() => null);
		if (session === this.session && selection === this.selection) this.detail = detail;
	}

	setQuery(query: string) {
		this.query = query;
	}

	/** Arrow navigation in the entry list. */
	moveSelection(delta: number) {
		const list = this.visibleEntries;
		if (!list.length) return;
		const index = list.findIndex((e) => e.id === this.selectedId);
		const next = list[Math.min(list.length - 1, Math.max(0, index < 0 ? 0 : index + delta))];
		if (next.id !== this.selectedId) void this.selectEntry(next.id);
	}

	// ---------------------------------------------------------------- secrets

	async toggleReveal() {
		if (this.revealed !== null) {
			this.revealed = null;
			return;
		}
		const id = this.selectedId;
		if (!id) return;
		const selection = this.selection;
		const session = this.session;
		const value = await this.api.vault.revealField(id, "password").catch(() => null);
		if (session === this.session && selection === this.selection && this.selectedId === id) {
			this.revealed = value;
		}
	}

	async copy(field: Field) {
		const id = this.selectedId;
		if (!id || this.screen !== "vault") return;
		const session = this.session;
		let secs: number | null;
		try {
			secs = await this.api.vault.copyField(id, field);
		} catch {
			return;
		}
		if (session !== this.session) return;
		this.toast = { field, secs: secs ?? this.settings.clipboard_clear_secs };
		this.startToastTimer();
	}

	private startToastTimer() {
		this.stopToastTimer();
		this.toastTimer = setInterval(() => {
			if (!this.toast || this.toast.secs <= 1) {
				this.toast = null;
				this.stopToastTimer();
			} else {
				this.toast = { ...this.toast, secs: this.toast.secs - 1 };
			}
		}, 1000);
	}

	private stopToastTimer() {
		if (this.toastTimer) clearInterval(this.toastTimer);
		this.toastTimer = null;
	}

	// ---------------------------------------------------------------- editor

	private defaultGroup(): GroupId {
		if (this.groupId !== ALL && this.groupId !== this.recycleBinId) return this.groupId;
		return this.groups.find((g) => !g.isRecycleBin)?.id ?? this.rootId;
	}

	newEntry() {
		if (this.screen !== "vault") return;
		return this.requestLeave(() => {
			this.selection++;
			const draft: EntryDraft = {
				id: null,
				groupId: this.defaultGroup(),
				title: "",
				username: "",
				password: "",
				url: "",
				notes: "",
			};
			this.editor = { draft, original: { ...draft }, isNew: true };
		});
	}

	editEntry(id = this.selectedId) {
		if (!id || this.screen !== "vault" || this.inRecycleBin) return;
		if (this.editor && !this.editor.isNew && this.editor.draft.id === id) return;
		return this.requestLeave(async () => {
			const selection = ++this.selection;
			const session = this.session;
			const draft = await this.api.vault.getEntryForEdit(id).catch(() => null);
			if (!draft || session !== this.session || selection !== this.selection) return;
			this.selectedId = id;
			this.revealed = null;
			this.editor = { draft, original: { ...draft }, isNew: false };
		});
	}

	updateDraft(patch: Partial<EntryDraft>) {
		if (!this.editor || this.savingEntry) return;
		this.editor = { ...this.editor, draft: { ...this.editor.draft, ...patch } };
	}

	/** Esc / Cancel: asks first when there are changes. */
	cancelEditor() {
		if (!this.editor) return;
		void this.requestLeave(() => {});
	}

	async saveEditor(): Promise<boolean> {
		const editor = this.editor;
		if (!editor || !editor.draft.title.trim() || this.savingEntry) return false;
		this.savingEntry = true;
		try {
			return await this.track(this.saveEditorOnce(editor));
		} finally {
			this.savingEntry = false;
		}
	}

	/**
	 * Registers vault work so that leaving waits for it. The vault counts as
	 * changed from the start: until the backend says otherwise (a save, or
	 * `databaseInfo` when leaving), nothing is assumed written.
	 */
	private track<T>(work: Promise<T>): Promise<T> {
		this.vaultDirty = true;
		this.pending.add(work);
		void work.then(
			() => this.pending.delete(work),
			() => this.pending.delete(work),
		);
		return work;
	}

	/** A change: the backend call, a refresh of the lists, then the save. */
	private change(op: (session: number) => Promise<unknown>): Promise<boolean> {
		return this.track(
			(async () => {
				const session = this.session;
				try {
					await op(session);
				} catch {
					return false;
				}
				// A failed refresh must not skip the save.
				await this.refresh(session).catch(() => {});
				if (session !== this.session) return false;
				return this.persist();
			})(),
		);
	}

	private async saveEditorOnce(editor: EditorState): Promise<boolean> {
		const session = this.session;
		const selection = this.selection;
		let id: EntryId;
		try {
			this.vaultDirty = true;
			id = await this.api.vault.saveEntry({
				...editor.draft,
				title: editor.draft.title.trim(),
			});
		} catch {
			return false;
		}
		if (session !== this.session || selection !== this.selection) return false;
		const after = ++this.selection;
		this.editor = null;
		this.generated = "";
		if (this.overlay === "generator") this.overlay = null;
		await this.refresh(session).catch(() => {});
		if (session !== this.session) return false;
		// Show the saved entry unless the user has moved on meanwhile.
		if (after === this.selection) {
			if (this.groupId !== ALL && this.groupId !== editor.draft.groupId) this.groupId = ALL;
			this.query = "";
			await this.loadEntry(id);
		}
		if (session !== this.session) return false;
		return this.persist();
	}

	// ---------------------------------------------------------------- entries and groups

	askDeleteEntry(id = this.selectedId) {
		const e = this.entries.find((x) => x.id === id);
		if (!e || this.editor) return;
		this.confirm = {
			kind: "deleteEntry",
			id: e.id,
			title: e.title,
			permanent: e.groupId === this.recycleBinId || !this.db?.recycleBin,
		};
	}

	async confirmDelete() {
		const c = this.confirm;
		this.confirm = null;
		if (!c || c.kind === "leave" || c.kind === "unsaved") return;
		await this.change(async () => {
			if (c.kind === "deleteEntry") {
				await this.api.vault.deleteEntry(c.id);
				if (this.selectedId === c.id) this.clearSelection();
			} else {
				await this.api.vault.deleteGroup(c.id);
				if (this.groupId === c.id) this.groupId = ALL;
			}
		});
	}

	async restoreSelected() {
		const id = this.selectedId;
		if (!id || !this.inRecycleBin) return;
		await this.change(() => this.api.vault.restoreEntry(id));
	}

	async createGroup(name: string) {
		if (!name.trim() || !this.db) return;
		const parent =
			this.groupId === ALL || this.groupId === this.recycleBinId ? this.rootId : this.groupId;
		let created: GroupId | null = null;
		const session = this.session;
		await this.change(async () => {
			created = await this.api.vault.createGroup(parent, name);
		});
		if (created && session === this.session) await this.selectGroup(created);
	}

	startRenameGroup(id = this.groupId) {
		const g = this.groups.find((x) => x.id === id);
		if (g && !g.isRecycleBin) this.renamingGroupId = id;
	}

	async renameGroup(id: GroupId, name: string) {
		this.renamingGroupId = null;
		const g = this.groups.find((x) => x.id === id);
		if (!g || !name.trim() || name.trim() === g.name) return;
		await this.change(() => this.api.vault.renameGroup(id, name));
	}

	askDeleteGroup(id = this.groupId) {
		const g = this.groups.find((x) => x.id === id);
		if (!g || g.isRecycleBin) return;
		void this.requestLeave(() => {
			this.confirm = { kind: "deleteGroup", id: g.id, name: g.name };
		});
	}

	private async refresh(session: number) {
		const [tree, entries] = await Promise.all([
			this.api.vault.tree(),
			this.api.vault.listEntries(),
		]);
		if (session !== this.session) return;
		this.tree = tree;
		this.entries = entries;
	}

	// ---------------------------------------------------------------- saving

	/** Writes the file; true only if it was written for the current session. */
	persist(overwrite = false): Promise<boolean> {
		if (!this.db) return Promise.resolve(false);
		this.vaultDirty = true;
		const run = this.write(overwrite);
		this.saving = run;
		void run.finally(() => {
			if (this.saving === run) this.saving = null;
		});
		return run;
	}

	private async write(overwrite: boolean): Promise<boolean> {
		const session = this.session;
		this.saveState = "saving";
		this.lastSaveError = null;
		try {
			const info = overwrite
				? await this.api.vault.saveOverwrite()
				: await this.api.vault.save();
			if (session !== this.session) return false;
			this.db = info;
			this.vaultDirty = info.dirty;
			this.saveState = "saved";
			if (this.banner) this.banner = null;
			return true;
		} catch (e) {
			if (session !== this.session) return false;
			this.saveState = "failed";
			const kind = e instanceof VaultError ? e.kind : "io";
			this.lastSaveError = kind;
			if (kind === "notDurable") {
				// Written and renamed, but not confirmed on disk: the state
				// follows the file; the banner offers to write it again.
				this.vaultDirty = false;
				this.banner = "saveFailed";
			} else {
				this.banner = kind === "externalChange" ? "externalChange" : "saveFailed";
			}
			return false;
		}
	}

	/** "Reload": the file as another app left it; unsaved changes are dropped. */
	async reloadExternal() {
		const session = this.session;
		this.banner = null;
		try {
			const info = await this.api.vault.reload();
			if (session !== this.session) return;
			this.db = info;
			this.vaultDirty = info.dirty;
			this.saveState = "saved";
		} catch {
			if (session === this.session) this.banner = "saveFailed";
			return;
		}
		this.closeEditor();
		this.clearSelection();
		await this.refresh(session);
	}

	/** "Keep mine": write over what the other app saved. */
	async keepMine() {
		this.banner = null;
		await this.persist(true);
	}

	/** Shows the banner if another app changed the file (window focus). */
	async checkExternal() {
		if (this.screen !== "vault" || this.banner) return;
		const session = this.session;
		const unchanged = await this.api.vault.checkFile().catch(() => true);
		if (!unchanged && session === this.session && !this.banner) {
			this.banner = "externalChange";
		}
	}

	/**
	 * Runs `action` once nothing would be lost: a modified editor is saved or
	 * discarded, a save in flight finishes, and changes the last save could
	 * not write are retried or discarded. Lock does not come through here.
	 */
	requestLeaveVault(action: () => void | Promise<void>) {
		return this.requestLeave(async () => {
			// Changes still in flight finish first, then the backend has the
			// last word on whether anything is unsaved.
			while (this.pending.size || this.saving) {
				await Promise.allSettled([...this.pending, this.saving]);
			}
			if (this.screen === "vault") {
				const info = await this.api.vault.databaseInfo().catch(() => null);
				if (info) this.vaultDirty = info.dirty;
			}
			if (this.screen === "vault" && this.vaultDirty) {
				this.confirm = { kind: "unsaved", action };
				return;
			}
			await action();
		});
	}

	async resolveUnsaved(choice: "retry" | "discard" | "cancel") {
		const pending = this.confirm;
		if (pending?.kind !== "unsaved") return;
		this.confirm = null;
		if (choice === "cancel") return;
		if (choice === "retry" && !(await this.persist())) return;
		await pending.action();
	}

	// ---------------------------------------------------------------- database settings

	async updateDatabase(patch: DatabasePatch) {
		if (!this.db) return;
		await this.change(async (session) => {
			const info = await this.api.vault.updateDatabase(patch);
			if (session === this.session) this.db = info;
		});
	}

	/**
	 * New master password. The outcome says what the file now uses:
	 * - "changed": saved with the new password;
	 * - "notDurable": written with the new password, not confirmed on disk;
	 * - "pending": not saved; the new password applies at the next successful
	 *   save, unless the change is discarded (Reload);
	 * - "rejected": nothing changed, the old password stays.
	 */
	async changeMasterPassword(
		password: string,
	): Promise<"changed" | "notDurable" | "pending" | "rejected"> {
		if (!this.db || !password) return "rejected";
		const session = this.session;
		let accepted = false;
		const saved = await this.change(async () => {
			await this.api.vault.changeMasterPassword(password);
			accepted = true;
		});
		if (!accepted || session !== this.session) return "rejected";
		if (saved) return "changed";
		return this.lastSaveError === "notDurable" ? "notDurable" : "pending";
	}

	/** Drops the unsaved changes, a pending new master password included. */
	async discardUnsaved() {
		await this.reloadExternal();
	}

	async openUrl(url: string) {
		await this.api.vault.openUrl(url).catch(() => {});
	}

	async benchmark(memoryMib: number, parallelism: number): Promise<number | null> {
		const session = this.session;
		this.busy = "benchmark";
		try {
			const iterations = await this.api.vault.benchmarkKdf(memoryMib, parallelism);
			return session === this.session ? iterations : null;
		} finally {
			if (session === this.session) this.busy = null;
		}
	}

	// ---------------------------------------------------------------- app settings

	async updateSettings(patch: Partial<Settings>) {
		// Optimistic: the UI shows the change at once, then the writes go out
		// in order, each carrying the latest desired settings.
		this.settings = { ...this.settings, ...patch };
		const write = ++this.settingsWrite;
		const run = this.settingsQueue.then(async () => {
			// Plain data: $state proxies cannot cross IPC or structuredClone.
			const desired = $state.snapshot(this.settings) as Settings;
			let saved: Settings;
			try {
				saved = await this.api.app.updateSettings(desired);
			} catch {
				saved = await this.api.app.getSettings();
			}
			// A newer edit is queued: its own answer will settle the state.
			if (write === this.settingsWrite) this.settings = saved;
		});
		this.settingsQueue = run.catch(() => {});
		await run;
		// New generator defaults apply to the generator straight away.
		if (patch.generator) this.generator = generatorFromSettings(this.settings);
	}

	// ---------------------------------------------------------------- generator

	async regenerate() {
		const request = ++this.generatorRequest;
		const session = this.session;
		const selection = this.selection;
		const options = $state.snapshot(this.generator) as GeneratorOptions;
		// No stale preview: "Use password" stays disabled until this answer.
		this.generated = "";
		const value = await this.api.vault.generatePassword(options);
		if (
			request === this.generatorRequest &&
			session === this.session &&
			selection === this.selection
		) {
			this.generated = value;
		}
	}

	setGenerator(patch: Partial<GeneratorOptions>) {
		const next = { ...this.generator, ...patch };
		if (!(next.lowercase || next.uppercase || next.digits || next.symbols))
			next.lowercase = true;
		next.length = Math.min(128, Math.max(8, Math.round(next.length)));
		this.generator = next;
		void this.regenerate();
	}

	openGenerator() {
		this.overlay = "generator";
		void this.regenerate();
	}

	useGenerated() {
		if (this.editor && this.generated) this.updateDraft({ password: this.generated });
		this.overlay = null;
	}

	// ---------------------------------------------------------------- overlays

	openSettings(tab: SettingsTab = "general") {
		this.settingsTab = tab;
		this.overlay = "settings";
	}

	closeOverlay() {
		this.overlay = null;
	}

	dismissToast() {
		this.toast = null;
		this.stopToastTimer();
	}

	// ---------------------------------------------------------------- commands

	async run(id: CommandId) {
		const inVault = this.screen === "vault";
		switch (id) {
			case "new_database":
				return inVault
					? this.requestLeaveVault(() => this.leaveVault("create"))
					: this.startCreate();
			case "open_database":
				return inVault
					? this.requestLeaveVault(() => this.leaveVault("open"))
					: this.openOtherFile();
			case "close_database":
				return inVault && this.requestLeaveVault(() => this.lock());
			case "save":
				if (this.editor) return this.saveEditor();
				return inVault && this.persist();
			case "lock":
				return inVault && this.lock();
			case "quit":
				return this.requestLeaveVault(() => this.api.app.exit());
			case "find":
				if (!inVault) return;
				this.overlay = null;
				this.focusSearch++;
				return;
			case "command_palette":
				this.overlay = this.overlay === "palette" ? null : "palette";
				return;
			case "new_entry":
				return this.newEntry();
			case "edit_entry":
				return this.editEntry();
			case "copy_username":
				return this.copy("username");
			case "copy_password":
				return this.copy("password");
			case "copy_url":
				return this.copy("url");
			case "delete_entry":
				return this.askDeleteEntry();
			case "new_group":
				if (!inVault) return;
				return this.requestLeave(() => {
					this.renamingGroupId = "new";
				});
			case "rename_group":
				return inVault && this.startRenameGroup();
			case "delete_group":
				return inVault && this.askDeleteGroup();
			case "password_generator":
				return this.openGenerator();
			case "settings":
				return this.openSettings("general");
			case "database_settings":
				return inVault && this.openSettings("database");
			case "keyboard_shortcuts":
				this.overlay = this.overlay === "shortcuts" ? null : "shortcuts";
				return;
			case "about":
				return this.openSettings("about");
			case "demo_external_change":
				if (this.isDemo && inVault && "externalChange" in this.api.vault) {
					(this.api.vault as { externalChange: boolean }).externalChange = true;
					await this.checkExternal();
				}
				return;
			case "demo_save_failure":
				if (this.isDemo && "failNextSave" in this.api.vault) {
					(this.api.vault as { failNextSave: boolean }).failNextSave = true;
					if (inVault) await this.persist();
				}
				return;
			case "demo_first_run":
				if (this.isDemo && "firstRun" in this.api.vault) {
					(this.api.vault as { firstRun: boolean }).firstRun = true;
					if (inVault) this.lock();
					await this.loadRecents();
				}
				return;
		}
	}

	private async leaveVault(next: "create" | "open") {
		this.lock();
		if (next === "create") this.startCreate();
		else await this.openOtherFile();
	}

	/** Window-level keydown: shortcuts from the command registry. */
	handleKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			if (this.confirm || this.overlay) return; // dialogs close themselves
			if (this.renamingGroupId) return;
			if (this.editor) {
				event.preventDefault();
				this.cancelEditor();
			}
			return;
		}
		const c = commandForKeydown(event);
		if (!c) return;
		// Plain shortcuts leave selected text alone (Delete, Ctrl+B…).
		if (c.scope === "outsideText" && globalThis.getSelection?.()?.toString()) return;
		if (c.demoOnly && !this.isDemo) return;
		// While a dialog is open only the dialog-independent shortcuts apply.
		if (this.confirm && c.id !== "lock") return;
		event.preventDefault();
		void this.run(c.id);
	}

	/**
	 * Copy Password is bound to the DOM `copy` event: the native Edit › Copy
	 * item and Ctrl/⌘+C both produce it on every platform, and it leaves
	 * copying selected text alone.
	 */
	handleCopy(event: ClipboardEvent) {
		if (this.screen !== "vault" || !this.selectedId || this.editor || this.confirm) return;
		if (isEditableTarget(event.target) || isEditableTarget(document.activeElement)) return;
		if (globalThis.getSelection?.()?.toString()) return;
		if (command("copy_password").trigger !== "copy") return;
		event.preventDefault();
		void this.copy("password");
	}
}

function generatorFromSettings(s: Settings): GeneratorOptions {
	return {
		length: s.generator.length,
		lowercase: s.generator.lowercase,
		uppercase: s.generator.uppercase,
		digits: s.generator.digits,
		symbols: s.generator.symbols,
		excludeAmbiguous: s.generator.exclude_ambiguous,
	};
}
