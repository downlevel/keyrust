import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

/** The app's Tauri identifier: its config dir is `$XDG_CONFIG_HOME/<identifier>`. */
export const IDENTIFIER = "io.github.downlevel.keyrust";
export const FIXTURE_PASSWORD = "keyrust-test-password";

const FIXTURES = resolve(import.meta.dirname, "../../../tests/fixtures");

/** Everything the test owns: copies of the fixtures and the app's XDG dirs. */
export function e2eDir(): string {
	return process.env.KEYRUST_E2E_DIR ?? join(tmpdir(), "keyrust-e2e");
}

export function paths() {
	const root = e2eDir();
	return {
		root,
		rich: join(root, "vaults", "keepassxc-rich.kdbx"),
		kdbx31: join(root, "vaults", "keepassxc-kdbx31.kdbx"),
		config: join(root, "config"),
		settings: join(root, "config", IDENTIFIER, "settings.json"),
		artifacts: join(root, "artifacts"),
	};
}

/**
 * Prepares a fresh directory the way a user would leave it: two databases in
 * the recent files. No test hook in the app: it reads its usual settings file,
 * found through XDG_CONFIG_HOME. The original fixtures are never touched.
 */
export function prepare() {
	const p = paths();
	rmSync(p.root, { recursive: true, force: true });
	mkdirSync(join(p.root, "vaults"), { recursive: true });
	mkdirSync(join(p.config, IDENTIFIER), { recursive: true });
	mkdirSync(p.artifacts, { recursive: true });
	copyFileSync(join(FIXTURES, "keepassxc-rich.kdbx"), p.rich);
	copyFileSync(join(FIXTURES, "keepassxc-kdbx31.kdbx"), p.kdbx31);
	// Same schema as keyrust_core::settings::Settings; missing fields (the
	// idle delay included) take their defaults.
	writeFileSync(
		p.settings,
		JSON.stringify({ version: 1, recent_files: [p.rich, p.kdbx31] }),
	);
	for (const dir of ["data", "cache"])
		mkdirSync(join(p.root, dir), { recursive: true });
	Object.assign(process.env, {
		XDG_CONFIG_HOME: p.config,
		XDG_DATA_HOME: join(p.root, "data"),
		XDG_CACHE_HOME: join(p.root, "cache"),
		KEYRUST_LOG: process.env.KEYRUST_LOG ?? "info",
	});
	return p;
}
