import type {
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
	StrengthEstimate,
} from "./types";

/** The `kind` of the Rust `IpcError` (plus none of the details). */
export type VaultErrorKind =
	| "wrongKey"
	| "invalidFormat"
	| "unsupportedVersion"
	| "externalChange"
	| "alreadyExists"
	| "notFound"
	| "invalid"
	| "locked"
	| "io"
	| "notDurable"
	| "tooExpensive"
	| "cancelled";

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
	/** Native open dialog; rejects with `cancelled`. */
	chooseFile(): Promise<RecentFile>;
	/** Native save dialog for a new database; rejects with `cancelled`. */
	chooseNewPath(name: string): Promise<RecentFile>;
	unlock(path: string, password: string): Promise<DatabaseInfo>;
	create(options: CreateOptions): Promise<DatabaseInfo>;
	lock(): Promise<void>;
	/** User input, at most every few seconds: restarts the backend's idle delay. */
	activity(): Promise<void>;
	databaseInfo(): Promise<DatabaseInfo>;
	tree(): Promise<GroupNode>;
	/** Every entry, recycle bin included. */
	listEntries(): Promise<EntrySummary[]>;
	getEntry(id: EntryId): Promise<EntryDetail>;
	getEntryForEdit(id: EntryId): Promise<EntryDraft>;
	revealField(id: EntryId, field: Field): Promise<string>;
	/** Copies on the backend side; resolves to the seconds until it is cleared. */
	copyField(id: EntryId, field: Field): Promise<number | null>;
	saveEntry(draft: EntryDraft): Promise<EntryId>;
	deleteEntry(id: EntryId): Promise<void>;
	restoreEntry(id: EntryId): Promise<void>;
	createGroup(parent: GroupId, name: string): Promise<GroupId>;
	renameGroup(id: GroupId, name: string): Promise<void>;
	deleteGroup(id: GroupId): Promise<void>;
	updateDatabase(patch: DatabasePatch): Promise<DatabaseInfo>;
	/** Applies with the next successful save. */
	changeMasterPassword(password: string): Promise<void>;
	/** Writes the file; rejects with `externalChange` if another app changed it. */
	save(): Promise<DatabaseInfo>;
	/** "Keep mine": writes over whatever is on disk. */
	saveOverwrite(): Promise<DatabaseInfo>;
	/** "Reload": re-reads the file, dropping unsaved changes. */
	reload(): Promise<DatabaseInfo>;
	/** Whether the file still holds what Keyrust last read or wrote. */
	checkFile(): Promise<boolean>;
	generatePassword(options: GeneratorOptions): Promise<string>;
	estimateStrength(password: string): Promise<StrengthEstimate>;
	/** Iterations that take about one second with these parameters. */
	benchmarkKdf(memoryMib: number, parallelism: number): Promise<number>;
	/** Opens an http(s) link in the system browser. */
	openUrl(url: string): Promise<void>;
}
