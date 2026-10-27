import { DEFAULT_SETTINGS } from "$lib/api/app";
import {
	VaultError,
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
	type KdfParams,
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
	| { kind: "leave"; action: () => void | Promise<void> };

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
	unlockError = $state<"wrongPassword" | "notFound" | null>(null);
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

	toast = $state<{ field: Field; secs: number } | null>(null);
	banner = $state<"externalChange" | "saveFailed" | null>(null);
	saveState = $state<"saved" | "saving" | "failed">("saved");

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
		const file = await this.api.vault.chooseFile();
		if (!file) return;
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

	async unlock(password: string) {
		const recent = this.recents[this.recentIndex];
		if (!recent || this.busy) return;
		const session = ++this.session;
		this.busy = "unlock";
		this.unlockError = null;
		try {
			const info = await this.api.vault.unlock(recent.path, password);
			if (session !== this.session) {
				void this.api.vault.lock();
				return;
			}
			await this.enterVault(info, session);
		} catch (e) {
			if (session !== this.session) return;
			this.unlockError =
				e instanceof VaultError && e.kind === "notFound" ? "notFound" : "wrongPassword";
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
		kdf: KdfParams;
	}) {
		if (this.busy) return;
		const session = ++this.session;
		this.busy = "create";
		this.createError = false;
		try {
			const info = await this.api.vault.create(options);
			if (session !== this.session) {
				void this.api.vault.lock();
				return;
			}
			this.recents = [
				{ name: info.name, path: info.path, missing: false },
				...this.recents.filter((r) => r.path !== info.path),
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
		void this.api.vault.lock();
		this.screen = this.recents.length ? "unlock" : "welcome";
	}

	// ---------------------------------------------------------------- leaving the editor

	/** Runs `action` once a modified editor has been saved or discarded. */
	async requestLeave(action: () => void | Promise<void>) {
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
		if (choice === "save" && !(await this.saveEditor())) return;
		this.closeEditor();
		await pending.action();
	}

	private closeEditor() {
		if (!this.editor) return;
		this.selection++;
		this.editor = null;
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
		await this.api.vault.copyField(id, field);
		if (session !== this.session) return;
		this.toast = { field, secs: this.settings.clipboard_clear_secs };
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
		if (!this.editor) return;
		this.editor = { ...this.editor, draft: { ...this.editor.draft, ...patch } };
	}

	/** Esc / Cancel: asks first when there are changes. */
	cancelEditor() {
		if (!this.editor) return;
		void this.requestLeave(() => {});
	}

	async saveEditor(): Promise<boolean> {
		const editor = this.editor;
		if (!editor || !editor.draft.title.trim()) return false;
		const session = this.session;
		const selection = this.selection;
		let id: EntryId;
		try {
			id = await this.api.vault.saveEntry({
				...editor.draft,
				title: editor.draft.title.trim(),
			});
		} catch {
			return false;
		}
		if (session !== this.session || selection !== this.selection) return false;
		this.selection++;
		this.editor = null;
		if (this.overlay === "generator") this.overlay = null;
		await this.refresh(session);
		if (session !== this.session) return false;
		if (this.groupId !== ALL && this.groupId !== editor.draft.groupId) this.groupId = ALL;
		this.query = "";
		await this.loadEntry(id);
		await this.persist();
		return true;
	}

	// ---------------------------------------------------------------- entries and groups

	askDeleteEntry(id = this.selectedId) {
		const e = this.entries.find((x) => x.id === id);
		if (!e || this.editor) return;
		this.confirm = {
			kind: "deleteEntry",
			id: e.id,
			title: e.title,
			permanent: e.groupId === this.recycleBinId,
		};
	}

	async confirmDelete() {
		const c = this.confirm;
		this.confirm = null;
		if (!c || c.kind === "leave") return;
		const session = this.session;
		if (c.kind === "deleteEntry") {
			await this.api.vault.deleteEntry(c.id);
			if (this.selectedId === c.id) this.clearSelection();
		} else {
			await this.api.vault.deleteGroup(c.id);
			if (this.groupId === c.id) this.groupId = ALL;
		}
		await this.refresh(session);
		await this.persist();
	}

	async restoreSelected() {
		const id = this.selectedId;
		if (!id || !this.inRecycleBin) return;
		const session = this.session;
		await this.api.vault.restoreEntry(id);
		await this.refresh(session);
		await this.persist();
	}

	async createGroup(name: string) {
		if (!name.trim() || !this.db) return;
		const session = this.session;
		const parent =
			this.groupId === ALL || this.groupId === this.recycleBinId ? this.rootId : this.groupId;
		const id = await this.api.vault.createGroup(parent, name);
		await this.refresh(session);
		if (session !== this.session) return;
		await this.selectGroup(id);
		await this.persist();
	}

	startRenameGroup(id = this.groupId) {
		const g = this.groups.find((x) => x.id === id);
		if (g && !g.isRecycleBin) this.renamingGroupId = id;
	}

	async renameGroup(id: GroupId, name: string) {
		this.renamingGroupId = null;
		const g = this.groups.find((x) => x.id === id);
		if (!g || !name.trim() || name.trim() === g.name) return;
		const session = this.session;
		await this.api.vault.renameGroup(id, name);
		await this.refresh(session);
		await this.persist();
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

	async persist() {
		if (!this.db) return;
		const session = this.session;
		this.saveState = "saving";
		try {
			await this.api.vault.save();
			if (session !== this.session) return;
			this.saveState = "saved";
			if (this.banner === "saveFailed") this.banner = null;
		} catch {
			if (session !== this.session) return;
			this.saveState = "failed";
			this.banner = "saveFailed";
		}
	}

	async reloadExternal() {
		this.banner = null;
		this.clearSelection();
		await this.refresh(this.session);
	}

	async keepMine() {
		this.banner = null;
		await this.persist();
	}

	// ---------------------------------------------------------------- database settings

	async updateDatabase(patch: Partial<Omit<DatabaseInfo, "path">>) {
		if (!this.db) return;
		const session = this.session;
		const info = await this.api.vault.updateDatabase(patch);
		if (session !== this.session) return;
		this.db = info;
		if (patch.name !== undefined) await this.refresh(session);
		await this.persist();
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
		// Plain data: $state proxies cannot cross IPC or structuredClone.
		const next = $state.snapshot({ ...this.settings, ...patch }) as Settings;
		this.settings = next;
		try {
			this.settings = await this.api.app.updateSettings(next);
		} catch {
			this.settings = await this.api.app.getSettings();
		}
	}

	// ---------------------------------------------------------------- generator

	async regenerate() {
		const session = this.session;
		const value = await this.api.vault.generatePassword(this.generator);
		if (session === this.session) this.generated = value;
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
					? this.requestLeave(() => this.leaveVault("create"))
					: this.startCreate();
			case "open_database":
				return inVault
					? this.requestLeave(() => this.leaveVault("open"))
					: this.openOtherFile();
			case "close_database":
				return inVault && this.requestLeave(() => this.lock());
			case "save":
				if (this.editor) return this.saveEditor();
				return inVault && this.persist();
			case "lock":
				return inVault && this.lock();
			case "quit":
				return this.requestLeave(() => this.api.app.exit());
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
				if (this.isDemo && inVault) this.banner = "externalChange";
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
