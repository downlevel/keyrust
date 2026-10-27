<script lang="ts">
	import { m } from "$lib/paraglide/messages.js";

	let { size = "md", subtitle }: { size?: "sm" | "md" | "lg"; subtitle?: string } = $props();

	const box = { sm: "size-6", md: "size-10", lg: "size-11" };
</script>

<div class="flex items-center gap-3">
	<!-- Same drawing as crates/app/icons/source.svg (the app icon). -->
	<svg viewBox="0 0 1024 1024" class="shrink-0 {box[size]}" aria-hidden="true">
		<rect width="1024" height="1024" rx="232" class="fill-primary" />
		<g
			fill="none"
			class="stroke-primary-foreground"
			stroke-width="92"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<circle cx="392" cy="404" r="150" />
			<path d="M498 510 L788 800" />
			<path d="M690 702 L772 620" />
			<path d="M590 602 L654 538" />
		</g>
	</svg>
	{#if size !== "sm"}
		<div class="flex flex-col">
			<span class="text-lg font-semibold tracking-tight">{m.app_name()}</span>
			{#if subtitle}<span class="text-xs text-muted-foreground">{subtitle}</span>{/if}
		</div>
	{/if}
</div>
