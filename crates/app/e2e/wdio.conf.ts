import { spawn, type ChildProcess } from "node:child_process";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { paths, prepare } from "./setup.ts";

// The real app (Rust backend, IPC, WebKitGTK) driven through tauri-driver,
// following the Tauri 2 "WebdriverIO" guide. Linux only: tauri-driver has no
// macOS support, and Windows needs a matching msedgedriver.

const APPLICATION = resolve(
	import.meta.dirname,
	"../../../target/debug/keyrust",
);
let driver: ChildProcess | undefined;

export const config: WebdriverIO.Config = {
	hostname: "127.0.0.1",
	port: 4444,
	specs: ["./specs/**/*.e2e.ts"],
	maxInstances: 1,
	capabilities: [
		{
			maxInstances: 1,
			// WebKitWebDriver speaks classic WebDriver only, not BiDi.
			"wdio:enforceWebDriverClassic": true,
			"tauri:options": { application: APPLICATION },
		} as WebdriverIO.Capabilities,
	],
	logLevel: "warn",
	waitforTimeout: 15_000,
	connectionRetryTimeout: 120_000,
	connectionRetryCount: 2,
	framework: "mocha",
	reporters: ["spec"],
	mochaOpts: { ui: "bdd", timeout: 120_000 },

	onPrepare: () => {
		const p = prepare();
		console.log(`e2e directory: ${p.root}`);
	},

	beforeSession: () => {
		// The app inherits this environment (XDG dirs, KEYRUST_LOG) from tauri-driver.
		driver = spawn(
			process.env.TAURI_DRIVER ??
				join(homedir(), ".cargo", "bin", "tauri-driver"),
			[],
			{
				stdio: [null, process.stdout, process.stderr],
				env: process.env,
			},
		);
	},

	before: async () => {
		// Collect script errors from the first moment, for the failure report.
		await browser.execute(() => {
			const w = window as unknown as { __errors: string[] };
			w.__errors = [];
			addEventListener("error", (e) =>
				w.__errors.push(String(e.message)),
			);
			addEventListener("unhandledrejection", (e) =>
				w.__errors.push(String(e.reason)),
			);
		});
	},

	afterTest: async (test, _context, { passed }) => {
		if (!passed) {
			// What the webview shows (no secret is on screen before an entry is
			// edited or revealed, and the fixture password is public anyway).
			const state = await browser
				.execute(() => ({
					url: location.href,
					readyState: document.readyState,
					tauri: "__TAURI_INTERNALS__" in window,
					errors:
						(window as unknown as { __errors?: string[] })
							.__errors ?? [],
					text:
						document.body?.innerText
							.replace(/\s+/g, " ")
							.slice(0, 300) ?? "",
				}))
				.catch((e: unknown) => ({ error: String(e) }));
			console.log(
				`page state after "${test.title}": ${JSON.stringify(state)}`,
			);
			// Did SvelteKit's inline bootstrap run (a CSP block would stop it),
			// and do its module imports load?
			const boot = await browser
				.execute(async () => {
					const kit = Object.keys(window).filter((k) =>
						k.startsWith("__sveltekit"),
					);
					const source = [...document.scripts]
						.map((s) => s.textContent ?? "")
						.join("\n");
					const imports = [...source.matchAll(/import\("([^"]+)"\)/g)].map(
						(m) => m[1],
					);
					const results: string[] = [];
					for (const url of imports) {
						try {
							await import(/* @vite-ignore */ url);
							results.push(`${url}: ok`);
						} catch (e) {
							results.push(`${url}: ${String(e)}`);
						}
					}
					return {
						kit,
						results,
						body: document.body?.innerHTML.slice(0, 400) ?? "",
					};
				})
				.catch((e: unknown) => ({ error: String(e) }));
			console.log(`page boot after "${test.title}": ${JSON.stringify(boot)}`);
			const name = test.title.replace(/[^\w-]+/g, "_");
			await browser.saveScreenshot(
				join(paths().artifacts, `${name}.png`),
			);
		}
	},

	afterSession: () => {
		driver?.kill();
	},
};
