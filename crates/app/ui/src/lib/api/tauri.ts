import { invoke } from "@tauri-apps/api/core";
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
import { VaultError, type VaultApi, type VaultErrorKind } from "./vault";

/** Rust commands answer errors as `{ kind }`; anything else is an I/O error. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (e) {
		const kind = (e as { kind?: VaultErrorKind } | null)?.kind;
		throw new VaultError(typeof kind === "string" ? kind : "io");
	}
}

/** The real vault, over the IPC commands of `crates/app/src/commands.rs`. */
export class TauriVault implements VaultApi {
	readonly isDemo = false;

	recentFiles() {
		return call<RecentFile[]>("recent_files");
	}
	chooseFile() {
		return call<RecentFile>("choose_database_file");
	}
	chooseNewPath(name: string) {
		return call<RecentFile>("choose_new_database_path", { name });
	}
	unlock(path: string, password: string) {
		return call<DatabaseInfo>("unlock", { path, password });
	}
	create(options: CreateOptions) {
		return call<DatabaseInfo>("create", { request: options });
	}
	lock() {
		return call<void>("lock_vault");
	}
	databaseInfo() {
		return call<DatabaseInfo>("database_info");
	}
	tree() {
		return call<GroupNode>("tree");
	}
	listEntries() {
		return call<EntrySummary[]>("list_entries");
	}
	getEntry(id: EntryId) {
		return call<EntryDetail>("get_entry", { id });
	}
	getEntryForEdit(id: EntryId) {
		return call<EntryDraft>("get_entry_for_edit", { id });
	}
	revealField(id: EntryId, field: Field) {
		return call<string>("reveal_field", { id, field });
	}
	copyField(id: EntryId, field: Field) {
		return call<number>("copy_field", { id, field });
	}
	saveEntry(draft: EntryDraft) {
		return call<EntryId>("save_entry", { draft });
	}
	deleteEntry(id: EntryId) {
		return call<void>("delete_entry", { id });
	}
	restoreEntry(id: EntryId) {
		return call<void>("restore_entry", { id });
	}
	createGroup(parent: GroupId, name: string) {
		return call<GroupId>("create_group", { parent, name });
	}
	renameGroup(id: GroupId, name: string) {
		return call<void>("rename_group", { id, name });
	}
	deleteGroup(id: GroupId) {
		return call<void>("delete_group", { id });
	}
	updateDatabase(patch: DatabasePatch) {
		return call<DatabaseInfo>("update_database", { patch });
	}
	changeMasterPassword(password: string) {
		return call<void>("change_master_password", { password });
	}
	save() {
		return call<DatabaseInfo>("save");
	}
	saveOverwrite() {
		return call<DatabaseInfo>("save_overwrite");
	}
	reload() {
		return call<DatabaseInfo>("reload");
	}
	checkFile() {
		return call<boolean>("check_file");
	}
	generatePassword(options: GeneratorOptions) {
		return call<string>("generate_password", { options });
	}
	estimateStrength(password: string) {
		return call<StrengthEstimate>("estimate_strength", { password });
	}
	benchmarkKdf(memoryMib: number, parallelism: number) {
		return call<number>("benchmark_kdf", { memoryMib, parallelism });
	}
	openUrl(url: string) {
		return call<void>("open_url", { url });
	}
}
