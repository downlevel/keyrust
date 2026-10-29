import { defineConfig, devices } from "@playwright/test";

/**
 * Accessibility audit and README screenshots on the static demo build in
 * Chromium. The real app (Rust backend + WebKitGTK) is covered by `../../e2e`.
 * The production CSP is not applied here: the Tauri webview enforces it.
 */
export default defineConfig({
	testDir: ".",
	// Screenshots run only through `pnpm screenshots`.
	testIgnore: process.env.KEYRUST_SCREENSHOTS ? [] : ["screenshots.spec.ts"],
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	reporter: process.env.CI ? [["list"], ["github"]] : "list",
	use: {
		baseURL: "http://127.0.0.1:4173",
		// The default Tauri window size.
		viewport: { width: 1100, height: 720 },
		trace: "retain-on-failure",
	},
	projects: [
		{
			name: "chromium",
			use: { ...devices["Desktop Chrome"], viewport: { width: 1100, height: 720 } },
		},
	],
	webServer: {
		command: "pnpm build && pnpm preview --host 127.0.0.1 --port 4173 --strictPort",
		url: "http://127.0.0.1:4173",
		reuseExistingServer: !process.env.CI,
		timeout: 180_000,
	},
});
