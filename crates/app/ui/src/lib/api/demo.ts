import { estimateStrength } from "./strength";
import type {
	Argon2idParams,
	CreateOptions,
	DatabaseInfo,
	DatabasePatch,
	EntryDetail,
	EntryDraft,
	EntryId,
	EntrySummary,
	Field,
	GeneratorOptions,
	GroupId,
	GroupNode,
	RecentFile,
} from "./types";
import { VaultError, type VaultApi } from "./vault";

// Sample data only: invented accounts, nothing real. The demo never touches
// the disk, the clipboard or browser storage.

interface DemoGroup {
	id: GroupId;
	parent: GroupId | null;
	name: string;
	isRecycleBin?: boolean;
}

interface DemoEntry extends EntryDraft {
	id: EntryId;
	created: string;
	modified: string;
	previousGroupId?: GroupId;
}

export const ROOT = "root";
export const RECYCLE_BIN = "recycle-bin";

const RECENTS: RecentFile[] = [
	{ name: "Personal", path: "~/Documents/Personal.kdbx", missing: false },
	{ name: "Work", path: "~/Nextcloud/Work.kdbx", missing: false },
	{ name: "Family", path: "~/Syncthing/Family.kdbx", missing: true },
];

const GROUPS: DemoGroup[] = [
	{ id: "email", parent: ROOT, name: "Email" },
	{ id: "work", parent: ROOT, name: "Work" },
	{ id: "dev", parent: "work", name: "Dev" },
	{ id: "banking", parent: ROOT, name: "Banking" },
	{ id: "social", parent: ROOT, name: "Social" },
	{ id: RECYCLE_BIN, parent: ROOT, name: "Recycle Bin", isRecycleBin: true },
];

const ENTRIES: DemoEntry[] = [
	entry(
		"github",
		"dev",
		"GitHub",
		"alex.rivera",
		"tW8#qLz!v2Rm9@kP4sXe",
		"https://github.com",
		"Recovery codes are in the attachment.\nTwo-factor: authenticator app.",
		"2026-01-12",
		"2026-10-03",
	),
	entry(
		"gitlab",
		"dev",
		"GitLab",
		"alex@company.example",
		"m3P$vR8!qXz2&Lw7",
		"https://gitlab.com",
		"",
		"2026-02-02",
		"2026-02-02",
	),
	entry(
		"jira",
		"work",
		"Jira",
		"alex.rivera",
		"Jr4!kV9#pT2s",
		"https://company.atlassian.net",
		"SSO through the company account.",
		"2026-02-20",
		"2026-08-14",
	),
	entry(
		"gmail",
		"email",
		"Gmail",
		"alex.rivera@gmail.com",
		"G7h!wQ2#zL9m@xR4",
		"https://mail.google.com",
		"",
		"2026-01-05",
		"2026-09-01",
	),
	entry(
		"proton",
		"email",
		"Proton Mail",
		"alexr",
		"pR0t#n!Mx8vQ2z",
		"https://proton.me",
		"",
		"2026-01-06",
		"2026-01-06",
	),
	entry(
		"bank",
		"banking",
		"Bank of Example",
		"4021 8834",
		"B4nk!Ex9#q",
		"https://bank.example",
		"Card PIN is not stored here.",
		"2026-01-10",
		"2026-07-22",
	),
	entry(
		"cu",
		"banking",
		"Credit Union",
		"alex.r",
		"cU7!mZ3#",
		"https://cu.example",
		"",
		"2026-01-11",
		"2026-01-11",
	),
	entry(
		"mastodon",
		"social",
		"Mastodon",
		"@alex@fosstodon.org",
		"M4st!d0n#Qx7vL",
		"https://fosstodon.org",
		"",
		"2026-03-03",
		"2026-03-03",
	),
	entry(
		"reddit",
		"social",
		"Reddit",
		"alex_r",
		"rd8!Vq2#mK",
		"https://reddit.com",
		"",
		"2026-03-04",
		"2026-09-30",
	),
	{
		...entry(
			"forum",
			RECYCLE_BIN,
			"Old forum account",
			"alexr99",
			"password1",
			"https://forum.example",
			"",
			"2026-01-01",
			"2026-09-15",
		),
		previousGroupId: "social",
	},
];

function entry(
	id: string,
	groupId: string,
	title: string,
	username: string,
	password: string,
	url: string,
	notes: string,
	created: string,
	modified: string,
): DemoEntry {
	return { id, groupId, title, username, password, url, notes, created, modified };
}

const SETS = {
	lowercase: "abcdefghijklmnopqrstuvwxyz",
	uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
	digits: "0123456789",
	symbols: "!#$%&*+-=?@^_~",
} as const;
const AMBIGUOUS = "Il1O0o";

export interface DemoOptions {
	/** Simulated latency of unlock, create and benchmark (ms). */
	delayMs?: number;
	/** Latency of every other call (ms). */
	callDelayMs?: number;
}

export class DemoVault implements VaultApi {
	readonly isDemo = true;
	/** Demo switches, driven by the "Demo: …" palette commands. */
	firstRun = false;
	failNextSave = false;
	externalChange = false;

	private delayMs: number;
	private callDelayMs: number;
	private info: DatabaseInfo | null = null;
	private groups: DemoGroup[] = [];
	private entries: DemoEntry[] = [];
	private nextId = 1;

	constructor(options: DemoOptions = {}) {
		this.delayMs = options.delayMs ?? 900;
		this.callDelayMs = options.callDelayMs ?? 0;
	}

	async recentFiles(): Promise<RecentFile[]> {
		await this.pause();
		return this.firstRun ? [] : RECENTS.map((r) => ({ ...r }));
	}

	async chooseFile(): Promise<RecentFile> {
		await this.pause();
		return { ...RECENTS[0] };
	}

	async chooseNewPath(name: string): Promise<RecentFile> {
		await this.pause();
		const clean = name.trim() || "Passwords";
		return { name: clean, path: `~/Documents/${clean}.kdbx`, missing: true };
	}

	async unlock(path: string, password: string): Promise<DatabaseInfo> {
		await this.pause(this.delayMs);
		const recent = RECENTS.find((r) => r.path === path);
		if (!recent || recent.missing) throw new VaultError("notFound");
		if (!password || password.toLowerCase() === "wrong") throw new VaultError("wrongKey");
		this.load(recent.name);
		return this.view();
	}

	async create(options: CreateOptions): Promise<DatabaseInfo> {
		await this.pause(this.delayMs);
		if (!options.password || !options.name.trim()) throw new VaultError("invalid");
		this.info = {
			name: options.name.trim(),
			cipher: options.cipher,
			kdf: argon2id(options.kdf),
			recycleBin: true,
			version: [4, 1],
			dirty: false,
		};
		this.groups = [{ id: RECYCLE_BIN, parent: ROOT, name: "Recycle Bin", isRecycleBin: true }];
		this.entries = [];
		return this.view();
	}

	async databaseInfo(): Promise<DatabaseInfo> {
		await this.pause();
		this.open();
		return this.view();
	}

	async lock(): Promise<void> {
		this.info = null;
		this.groups = [];
		this.entries = [];
	}
	/** The demo has no backend timer: activity is never reported to it. */
	async activity(): Promise<void> {}

	async tree(): Promise<GroupNode> {
		await this.pause();
		const info = this.open();
		const node = (g: DemoGroup | null): GroupNode => {
			const id = g ? g.id : ROOT;
			return {
				id,
				name: g ? g.name : info.name,
				isRecycleBin: !!g?.isRecycleBin,
				entryCount: this.entries.filter((e) => e.groupId === id).length,
				children: this.groups
					.filter((c) => c.parent === id)
					.sort((a, b) => Number(!!a.isRecycleBin) - Number(!!b.isRecycleBin))
					.map(node),
			};
		};
		return node(null);
	}

	async listEntries(): Promise<EntrySummary[]> {
		await this.pause();
		this.open();
		return this.entries.map(summary);
	}

	async getEntry(id: EntryId): Promise<EntryDetail> {
		await this.pause();
		const e = this.find(id);
		return {
			summary: summary(e),
			notes: e.notes,
			hasPassword: e.password.length > 0,
			created: e.created,
			modified: e.modified,
			historyLen: 0,
		};
	}

	async getEntryForEdit(id: EntryId): Promise<EntryDraft> {
		await this.pause();
		const e = this.find(id);
		return {
			id: e.id,
			groupId: e.groupId,
			title: e.title,
			username: e.username,
			password: e.password,
			url: e.url,
			notes: e.notes,
		};
	}

	async revealField(id: EntryId, field: Field): Promise<string> {
		await this.pause();
		return this.find(id)[field];
	}

	async copyField(id: EntryId): Promise<number | null> {
		// Simulated: the demo never writes to the system clipboard. The UI
		// uses its own clipboard setting for the countdown.
		await this.pause();
		this.find(id);
		return null;
	}

	async saveEntry(draft: EntryDraft): Promise<EntryId> {
		await this.pause();
		this.open();
		if (this.info) this.info.dirty = true;
		if (!draft.title.trim()) throw new VaultError("invalid");
		const today = new Date().toISOString().slice(0, 10);
		if (draft.id) {
			const e = this.find(draft.id);
			Object.assign(e, { ...draft, id: e.id, modified: today });
			return e.id;
		}
		const id = `new-${this.nextId++}`;
		this.entries.push({ ...draft, id, created: today, modified: today });
		return id;
	}

	async deleteEntry(id: EntryId): Promise<void> {
		await this.pause();
		if (this.info) this.info.dirty = true;
		const e = this.find(id);
		if (this.inBin(e.groupId) || !this.open().recycleBin) {
			this.entries = this.entries.filter((x) => x.id !== id);
		} else {
			e.previousGroupId = e.groupId;
			e.groupId = RECYCLE_BIN;
		}
	}

	async restoreEntry(id: EntryId): Promise<void> {
		await this.pause();
		if (this.info) this.info.dirty = true;
		const e = this.find(id);
		const target = this.groups.find((g) => g.id === e.previousGroupId && !g.isRecycleBin);
		e.groupId = target ? target.id : ROOT;
		e.previousGroupId = undefined;
	}

	async createGroup(parent: GroupId, name: string): Promise<GroupId> {
		await this.pause();
		this.open();
		if (this.info) this.info.dirty = true;
		if (!name.trim()) throw new VaultError("invalid");
		const id = `group-${this.nextId++}`;
		this.groups.push({ id, parent: parent === RECYCLE_BIN ? ROOT : parent, name: name.trim() });
		return id;
	}

	async renameGroup(id: GroupId, name: string): Promise<void> {
		await this.pause();
		if (this.info) this.info.dirty = true;
		const g = this.groups.find((x) => x.id === id && !x.isRecycleBin);
		if (!g || !name.trim()) throw new VaultError("invalid");
		g.name = name.trim();
	}

	/** Like the core: into the recycle bin whole, or permanently from there. */
	async deleteGroup(id: GroupId): Promise<void> {
		await this.pause();
		if (this.info) this.info.dirty = true;
		const g = this.groups.find((x) => x.id === id && !x.isRecycleBin);
		if (!g) throw new VaultError("invalid");
		if (this.open().recycleBin && !this.inBin(g.id)) {
			g.parent = RECYCLE_BIN;
			return;
		}
		const doomed = new Set<GroupId>();
		const collect = (gid: GroupId) => {
			doomed.add(gid);
			this.groups.filter((x) => x.parent === gid).forEach((x) => collect(x.id));
		};
		collect(id);
		this.entries = this.entries.filter((e) => !doomed.has(e.groupId));
		this.groups = this.groups.filter((x) => !doomed.has(x.id));
	}

	/** The recycle bin or a group inside it. */
	private inBin(groupId: GroupId): boolean {
		let current: GroupId | null = groupId;
		while (current) {
			if (current === RECYCLE_BIN) return true;
			current = this.groups.find((g) => g.id === current)?.parent ?? null;
		}
		return false;
	}

	async updateDatabase(patch: DatabasePatch): Promise<DatabaseInfo> {
		await this.pause();
		const info = this.open();
		if (patch.name !== undefined) info.name = patch.name;
		if (patch.cipher !== undefined) info.cipher = patch.cipher;
		if (patch.recycleBin !== undefined) info.recycleBin = patch.recycleBin;
		if (patch.kdf !== undefined) info.kdf = argon2id(patch.kdf);
		info.dirty = true;
		return this.view();
	}

	async changeMasterPassword(password: string): Promise<void> {
		await this.pause();
		if (!password) throw new VaultError("invalid");
		this.open().dirty = true;
	}

	async save(): Promise<DatabaseInfo> {
		await this.pause();
		const info = this.open();
		if (this.externalChange) throw new VaultError("externalChange");
		if (this.failNextSave) {
			this.failNextSave = false;
			throw new VaultError("io");
		}
		info.dirty = false;
		return this.view();
	}

	async saveOverwrite(): Promise<DatabaseInfo> {
		await this.pause();
		this.externalChange = false;
		this.open().dirty = false;
		return this.view();
	}

	async reload(): Promise<DatabaseInfo> {
		await this.pause();
		const info = this.open();
		this.externalChange = false;
		this.load(info.name);
		return this.view();
	}

	async checkFile(): Promise<boolean> {
		await this.pause();
		return !this.externalChange;
	}

	async openUrl(): Promise<void> {
		// The demo never leaves the page.
	}

	async generatePassword(options: GeneratorOptions): Promise<string> {
		await this.pause();
		let chars = "";
		for (const key of ["lowercase", "uppercase", "digits", "symbols"] as const) {
			if (options[key]) chars += SETS[key];
		}
		if (options.excludeAmbiguous)
			chars = [...chars].filter((c) => !AMBIGUOUS.includes(c)).join("");
		if (!chars) chars = SETS.lowercase;
		const length = Math.min(128, Math.max(8, Math.round(options.length)));
		const random = new Uint32Array(length);
		crypto.getRandomValues(random);
		return Array.from(random, (n) => chars[n % chars.length]).join("");
	}

	async estimateStrength(password: string) {
		await this.pause();
		return estimateStrength(password);
	}

	async benchmarkKdf(memoryMib: number, parallelism: number): Promise<number> {
		await this.pause(this.delayMs);
		// Plausible numbers for a laptop: more memory, fewer iterations per second.
		return Math.max(2, Math.round((448 / Math.max(8, memoryMib)) * Math.min(parallelism, 4)));
	}

	private load(name: string) {
		this.info = {
			name,
			cipher: "aes256",
			kdf: argon2id({ memoryMib: 64, iterations: 7, parallelism: 2 }),
			recycleBin: true,
			version: [4, 1],
			dirty: false,
		};
		this.groups = GROUPS.map((g) => ({ ...g }));
		this.entries = ENTRIES.map((e) => ({ ...e }));
	}

	private view(): DatabaseInfo {
		const info = this.open();
		return { ...info, kdf: { ...info.kdf }, version: [...info.version] };
	}

	private open(): DatabaseInfo {
		if (!this.info) throw new VaultError("locked");
		return this.info;
	}

	private find(id: EntryId): DemoEntry {
		this.open();
		const e = this.entries.find((x) => x.id === id);
		if (!e) throw new VaultError("notFound");
		return e;
	}

	private pause(ms = this.callDelayMs): Promise<void> {
		return ms > 0 ? new Promise((resolve) => setTimeout(resolve, ms)) : Promise.resolve();
	}
}

function argon2id(p: Argon2idParams): DatabaseInfo["kdf"] {
	return {
		kind: "argon2",
		variant: "argon2id",
		memoryBytes: p.memoryMib * 1024 * 1024,
		iterations: p.iterations,
		parallelism: p.parallelism,
	};
}

function summary(e: DemoEntry): EntrySummary {
	return { id: e.id, groupId: e.groupId, title: e.title, username: e.username, url: e.url };
}
