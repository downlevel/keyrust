import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import ts from "typescript-eslint";
import svelteConfig from "./svelte.config.js";

export default ts.config(
	{
		ignores: [
			".svelte-kit/",
			"build/",
			"node_modules/",
			"src/lib/paraglide/",
			"test-results/",
			"playwright-report/",
		],
	},
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs.recommended,
	prettier,
	...svelte.configs.prettier,
	{
		languageOptions: { globals: { ...globals.browser, ...globals.node } },
	},
	{
		files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
		languageOptions: {
			parserOptions: {
				projectService: true,
				extraFileExtensions: [".svelte"],
				parser: ts.parser,
				svelteConfig,
			},
		},
	},
	{
		// Components vendored from shadcn-svelte: they accept arbitrary hrefs by design.
		files: ["src/lib/components/ui/**"],
		rules: { "svelte/no-navigation-without-resolve": "off" },
	},
);
