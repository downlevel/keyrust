import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { svelteTesting } from "@testing-library/svelte/vite";
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
	plugins: [
		tailwindcss(),
		sveltekit(),
		paraglideVitePlugin({
			project: "./project.inlang",
			outdir: "./src/lib/paraglide",
			// Single-page app without locale URLs. Add "preferredLanguage" before
			// "baseLocale" once a second language ships.
			strategy: ["baseLocale"],
		}),
		svelteTesting(),
	],

	test: {
		environment: "jsdom",
		setupFiles: ["./src/tests/setup.ts"],
		include: ["src/**/*.test.ts"],
	},

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent Vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || "127.0.0.1",
		hmr: host
			? {
					protocol: "ws",
					host,
					port: 1421,
				}
			: undefined,
		watch: {
			// 3. the Rust crate lives one level up; never watch build output
			ignored: ["**/target/**"],
		},
	},
}));
