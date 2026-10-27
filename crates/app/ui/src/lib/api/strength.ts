import type { StrengthEstimate, StrengthLevel } from "./types";

/**
 * Character-pool entropy estimate used by the demo. Milestone 3 replaces it
 * with the core's estimator; the levels and thresholds stay the same.
 */
export function estimateStrength(password: string): StrengthEstimate {
	if (!password) return { level: 0, bits: 0 };
	let pool = 0;
	if (/[a-z]/.test(password)) pool += 26;
	if (/[A-Z]/.test(password)) pool += 26;
	if (/[0-9]/.test(password)) pool += 10;
	if (/[^a-zA-Z0-9]/.test(password)) pool += 32;
	const bits = Math.round([...password].length * Math.log2(pool));
	const level: StrengthLevel = bits < 40 ? 1 : bits < 60 ? 2 : bits < 80 ? 3 : 4;
	return { level, bits };
}
