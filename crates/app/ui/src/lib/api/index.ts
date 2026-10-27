// The single place that picks the API implementations: the real vault inside
// Tauri, the demo in the web preview and in tests.
import { isTauri } from "@tauri-apps/api/core";
import { defaultAppApi, type AppApi } from "./app";
import { DemoVault } from "./demo";
import { TauriVault } from "./tauri";
import type { VaultApi } from "./vault";

export interface Apis {
	vault: VaultApi;
	app: AppApi;
}

export function createApis(): Apis {
	return { vault: isTauri() ? new TauriVault() : new DemoVault(), app: defaultAppApi() };
}

export type { AppApi } from "./app";
export type { VaultApi } from "./vault";
export { VaultError, type VaultErrorKind } from "./vault";
export * from "./types";
