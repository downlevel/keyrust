import { invoke, isTauri } from "@tauri-apps/api/core";
import type { AppInfo, Argon2idParams, Settings } from "./types";

/** Application-level calls, backed by the Milestone 1 Tauri commands. */
export interface AppApi {
	appInfo(): Promise<AppInfo | null>;
	getSettings(): Promise<Settings>;
	updateSettings(settings: Settings): Promise<Settings>;
	/** Ends the application (after the UI has handled unsaved changes). */
	exit(): Promise<void>;
}

export const DEFAULT_SETTINGS: Settings = {
	version: 1,
	recent_files: [],
	clipboard_clear_secs: 20,
	idle_lock_mins: 5,
	lock_on_screen_lock: true,
	lock_on_minimize: false,
	generator: {
		length: 20,
		lowercase: true,
		uppercase: true,
		digits: true,
		symbols: true,
		exclude_ambiguous: false,
	},
};

const clamp = (value: number, min: number, max: number) =>
	Math.min(max, Math.max(min, Math.round(Number.isFinite(value) ? value : min)));

/** Same rules as `Settings::normalized` in keyrust-core. */
export function normalizeSettings(s: Settings): Settings {
	const g = { ...s.generator, length: clamp(s.generator.length, 8, 128) };
	if (!(g.lowercase || g.uppercase || g.digits || g.symbols)) g.lowercase = true;
	return {
		...s,
		version: 1,
		recent_files: [...new Set(s.recent_files)].slice(0, 10),
		clipboard_clear_secs: clamp(s.clipboard_clear_secs, 5, 300),
		idle_lock_mins: clamp(s.idle_lock_mins, 1, 240),
		generator: g,
	};
}

/**
 * The Argon2id parameters Keyrust creates and opens: the same bounds as
 * `kdf::validate` and the open limits in keyrust-core.
 */
export const KDF_LIMITS = {
	memoryMib: { min: 8, max: 2048 },
	iterations: { min: 1, max: 1000 },
	parallelism: { min: 1, max: 256 },
} as const;

/** Brings typed-in Argon2id parameters within `KDF_LIMITS`. */
export function clampKdf(p: Argon2idParams): Argon2idParams {
	const l = KDF_LIMITS;
	return {
		memoryMib: clamp(p.memoryMib, l.memoryMib.min, l.memoryMib.max),
		iterations: clamp(p.iterations, l.iterations.min, l.iterations.max),
		parallelism: clamp(p.parallelism, l.parallelism.min, l.parallelism.max),
	};
}

export class TauriAppApi implements AppApi {
	appInfo() {
		return invoke<AppInfo>("app_info");
	}
	getSettings() {
		return invoke<Settings>("get_settings");
	}
	updateSettings(settings: Settings) {
		return invoke<Settings>("update_settings", { settings });
	}
	exit() {
		return invoke<void>("exit_app");
	}
}

/** Outside Tauri (web preview, tests): settings live in memory only. */
export class MemoryAppApi implements AppApi {
	private settings: Settings = structuredClone(DEFAULT_SETTINGS);

	async appInfo() {
		return null;
	}
	async getSettings() {
		return structuredClone(this.settings);
	}
	async updateSettings(settings: Settings) {
		this.settings = normalizeSettings(structuredClone(settings));
		return structuredClone(this.settings);
	}
	async exit() {}
}

export function defaultAppApi(): AppApi {
	return isTauri() ? new TauriAppApi() : new MemoryAppApi();
}
