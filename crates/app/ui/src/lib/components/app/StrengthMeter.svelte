<script lang="ts">
	import type { StrengthEstimate } from "$lib/api";
	import { m } from "$lib/paraglide/messages.js";

	let {
		estimate,
		id,
	}: {
		estimate: StrengthEstimate;
		/** For `aria-describedby` on the password field it rates. */
		id?: string;
	} = $props();

	const labels = [
		m.strength_none,
		m.strength_weak,
		m.strength_fair,
		m.strength_good,
		m.strength_strong,
	];
	const fill = ["", "bg-strength-1", "bg-strength-2", "bg-strength-3", "bg-strength-4"];
	const text = [
		"text-muted-foreground",
		"text-destructive",
		"text-warning-soft-foreground",
		"text-success-text",
		"text-success-text",
	];

	const caption = $derived(
		estimate.level
			? m.strength_bits({ label: labels[estimate.level](), bits: String(estimate.bits) })
			: labels[0](),
	);
</script>

<div class="flex items-center gap-2.5" data-testid="strength-meter" data-level={estimate.level}>
	<div class="grid flex-1 grid-cols-4 gap-1" aria-hidden="true">
		{#each [1, 2, 3, 4] as segment (segment)}
			<span
				class="h-1 rounded-full {segment <= estimate.level
					? fill[estimate.level]
					: 'bg-border'}"
			></span>
		{/each}
	</div>
	<span {id} class="min-w-28 text-right text-xs font-medium {text[estimate.level]}"
		>{caption}</span
	>
</div>
