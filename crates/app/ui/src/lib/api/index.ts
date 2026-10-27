// The single place that picks the API implementations. Milestone 3 swaps
// DemoVault for the Tauri-backed vault here and nowhere else.
import { defaultAppApi, type AppApi } from "./app";
import { DemoVault } from "./demo";
import type { VaultApi } from "./vault";

export interface Apis {
	vault: VaultApi;
	app: AppApi;
}

export function createApis(): Apis {
	return { vault: new DemoVault(), app: defaultAppApi() };
}

export type { AppApi } from "./app";
export type { VaultApi } from "./vault";
export { VaultError } from "./vault";
export * from "./types";
