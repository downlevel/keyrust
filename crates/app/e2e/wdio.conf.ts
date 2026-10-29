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

	afterTest: async (test, _context, { passed }) => {
		if (!passed) {
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
