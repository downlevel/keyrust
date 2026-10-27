<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		label,
		children,
		actions,
		alwaysShowActions = false,
		multiline = false,
	}: {
		label: string;
		children: Snippet;
		actions?: Snippet;
		/** Secondary actions appear on hover / focus unless this is set. */
		alwaysShowActions?: boolean;
		multiline?: boolean;
	} = $props();
</script>

<div
	class="group flex gap-3 border-b border-muted py-1.5 pr-2 pl-4 last:border-b-0 hover:bg-surface {multiline
		? 'items-start py-3.5'
		: 'min-h-[52px] items-center'}"
	role="group"
	aria-label={label}
>
	<span class="w-24 shrink-0 text-[13px] text-muted-foreground">{label}</span>
	<div class="min-w-0 flex-1">{@render children()}</div>
	{#if actions}
		<div
			class="flex shrink-0 items-center gap-0.5 {alwaysShowActions
				? ''
				: 'opacity-0 group-focus-within:opacity-100 group-hover:opacity-100'}"
		>
			{@render actions()}
		</div>
	{/if}
</div>
