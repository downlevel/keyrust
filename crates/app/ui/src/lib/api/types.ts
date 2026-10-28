// Types shared by the UI and the API layer. They mirror the Rust core types of
// docs/ARCHITECTURE.md: list and detail views never carry a password value.

export type EntryId = string;
export type GroupId = string;

export interface EntrySummary {
	id: EntryId;
	groupId: GroupId;
	title: string;
	username: string;
	url: string;
}

export interface EntryDetail {
	summary: EntrySummary;
	notes: string;
	hasPassword: boolean;
	/** ISO 8601 (UTC), when known. */
	created: string | null;
	modified: string | null;
	historyLen: number;
}

/** Editable copy of an entry, including its password: only for the editor. */
export interface EntryDraft {
	id: EntryId | null;
	groupId: GroupId;
	title: string;
	username: string;
	password: string;
	url: string;
	notes: string;
}

export interface GroupNode {
	id: GroupId;
	name: string;
	children: GroupNode[];
	entryCount: number;
	isRecycleBin: boolean;
}

export type Field = "username" | "password" | "url" | "notes";

export interface GeneratorOptions {
	length: number;
	lowercase: boolean;
	uppercase: boolean;
	digits: boolean;
	symbols: boolean;
	excludeAmbiguous: boolean;
}

export type StrengthLevel = 0 | 1 | 2 | 3 | 4;

export interface StrengthEstimate {
	level: StrengthLevel;
	bits: number;
}

/** Twofish and "other" are read and preserved, never offered for new databases. */
export type Cipher = "aes256" | "chaCha20" | "twofish" | "other";

/** The key derivation stored in the file, as it is (never converted silently). */
export type KdfInfo =
	| { kind: "aesKdf"; rounds: number }
	| {
			kind: "argon2";
			variant: "argon2d" | "argon2id";
			memoryBytes: number;
			iterations: number;
			parallelism: number;
	  }
	| { kind: "other" };

/** Argon2id parameters Keyrust creates and edits. */
export interface Argon2idParams {
	memoryMib: number;
	iterations: number;
	parallelism: number;
}

export interface DatabaseInfo {
	name: string;
	cipher: Cipher;
	kdf: KdfInfo;
	recycleBin: boolean;
	/** KDBX version on disk, e.g. [4, 1]. */
	version: [number, number];
	/** Changes in memory that are not in the file yet. */
	dirty: boolean;
	/** Only from unlock / create: the backend generation of the new session. */
	session?: number;
}

export interface RecentFile {
	name: string;
	path: string;
	missing: boolean;
}

export interface CreateOptions {
	path: string;
	name: string;
	password: string;
	cipher: Cipher;
	kdf: Argon2idParams;
}

export interface DatabasePatch {
	name?: string;
	cipher?: Cipher;
	recycleBin?: boolean;
	kdf?: Argon2idParams;
}

/** `keyrust_core::settings::Settings`, as serialised by the Tauri commands. */
export interface Settings {
	version: number;
	recent_files: string[];
	clipboard_clear_secs: number;
	idle_lock_mins: number;
	lock_on_screen_lock: boolean;
	lock_on_minimize: boolean;
	generator: {
		length: number;
		lowercase: boolean;
		uppercase: boolean;
		digits: boolean;
		symbols: boolean;
		exclude_ambiguous: boolean;
	};
}

export interface AppInfo {
	version: string;
}

/** Why the backend locked the vault on its own (`locked` event). */
export type LockReason = "idle" | "screenLock" | "suspend" | "minimize";
