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
	/** ISO 8601 */
	created: string;
	/** ISO 8601 */
	modified: string;
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

export type Cipher = "aes256" | "chacha20";

export interface KdfParams {
	memoryMib: number;
	iterations: number;
	parallelism: number;
}

export interface DatabaseInfo {
	name: string;
	path: string;
	cipher: Cipher;
	kdf: KdfParams;
	recycleBin: boolean;
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
	kdf: KdfParams;
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
