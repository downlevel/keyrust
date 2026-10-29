import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, type TestInfo } from "@playwright/test";

/** The demo's master password (`src/lib/api/demo.ts`). */
export const DEMO_PASSWORD = "secret";

export async function openApp(page: Page) {
	await page.goto("/");
	await expect(page.getByLabel("Master password")).toBeFocused();
}

export async function unlock(page: Page) {
	await openApp(page);
	await page.getByLabel("Master password").fill(DEMO_PASSWORD);
	await page.getByRole("button", { name: "Unlock" }).click();
	await expect(page.getByRole("listbox", { name: /entries/i })).toBeVisible();
}

export async function palette(page: Page, label: string) {
	await page.keyboard.press("Control+k");
	const input = page.getByRole("combobox", { name: "Command palette" });
	await expect(input).toBeFocused();
	await input.fill(label);
	await page.getByRole("option", { name: label, exact: true }).click();
}

const IMPACT_FAILS = new Set(["serious", "critical"]);

/**
 * Runs axe (WCAG 2.0/2.1 A and AA) on the current page. Serious and critical
 * violations fail; moderate and minor ones are attached to the report and must
 * be fixed or justified in docs/ACCESSIBILITY.md.
 */
export async function audit(page: Page, testInfo: TestInfo, state: string) {
	const results = await new AxeBuilder({ page })
		.withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
		.analyze();
	const summary = results.violations.map((v) => ({
		id: v.id,
		impact: v.impact,
		help: v.help,
		targets: v.nodes.map((n) => n.target.join(" ")),
	}));
	if (summary.length) {
		await testInfo.attach(`axe-${state}.json`, {
			body: JSON.stringify(summary, null, 2),
			contentType: "application/json",
		});
		for (const v of summary) {
			const line = `[axe] ${state}: ${v.impact} ${v.id} — ${v.help} — ${v.targets.join(", ")}`;
			console.log(line);
			if (!IMPACT_FAILS.has(v.impact ?? ""))
				testInfo.annotations.push({ type: "axe", description: line });
		}
	}
	expect(
		summary.filter((v) => IMPACT_FAILS.has(v.impact ?? "")),
		`serious or critical axe violations in "${state}"`,
	).toEqual([]);
}
