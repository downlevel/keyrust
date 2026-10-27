import type {
	CreateOptions,
	DatabaseInfo,
	EntryDetail,
	EntryDraft,
	EntryId,
	EntrySummary,
	Field,
	GeneratorOptions,
	GroupId,
	GroupNode,
	RecentFile,
	StrengthEstimate,
} from "./types";

export type VaultErrorKind = "wrongPassword" | "notFound" | "locked" | "invalid" | "saveFailed";

/** Errors carry a kind for the UI to translate, never a secret or a path. */
export class VaultError extends Error {
	constructor(readonly kind: VaultErrorKind) {
		super(kind);
		this.name = "VaultError";
	}
}

/**
 * Everything the UI can ask of an open database. Milestone 3 implements it
 * over Tauri IPC; `DemoVault` implements it with sample data.
 *
 * Only `revealField` and `getEntryForEdit` return a password. `copyField`
 * copies on the backend side and returns nothing.
 */
export interface VaultApi {
	readonly isDemo: boolean;
	recentFiles(): Promise<RecentFile[]>;
	/** Chooses a file to open; `null` when the user cancels. */
	chooseFile(): Promise<RecentFile | null>;
	unlock(path: string, password: string): Promise<DatabaseInfo>;
	create(options: CreateOptions): Promise<DatabaseInfo>;
	lock(): Promise<void>;
	tree(): Promise<GroupNode>;
	/** Every entry, recycle bin included. */
	listEntries(): Promise<EntrySummary[]>;
	getEntry(id: EntryId): Promise<EntryDetail>;
	getEntryForEdit(id: EntryId): Promise<EntryDraft>;
	revealField(id: EntryId, field: Field): Promise<string>;
	copyField(id: EntryId, field: Field): Promise<void>;
	saveEntry(draft: EntryDraft): Promise<EntryId>;
	deleteEntry(id: EntryId): Promise<void>;
	restoreEntry(id: EntryId): Promise<void>;
	createGroup(parent: GroupId, name: string): Promise<GroupId>;
	renameGroup(id: GroupId, name: string): Promise<void>;
	deleteGroup(id: GroupId): Promise<void>;
	updateDatabase(patch: Partial<Omit<DatabaseInfo, "path">>): Promise<DatabaseInfo>;
	/** Writes the database file. */
	save(): Promise<void>;
	generatePassword(options: GeneratorOptions): Promise<string>;
	estimateStrength(password: string): Promise<StrengthEstimate>;
	/** Iterations that take about one second with these parameters. */
	benchmarkKdf(memoryMib: number, parallelism: number): Promise<number>;
}
