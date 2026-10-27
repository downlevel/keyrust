<script lang="ts">
	import FlaskConical from "@lucide/svelte/icons/flask-conical";
	import { m } from "$lib/paraglide/messages.js";

	let { detailed = false }: { detailed?: boolean } = $props();
</script>

<div class="flex items-center gap-2" data-testid="demo-badge">
	<span
		class="inline-flex items-center gap-1.5 rounded-full border border-warning-soft-border bg-warning-soft px-2 py-0.5 text-xs font-medium text-warning-soft-foreground"
		title={m.demo_explanation()}
	>
		<FlaskConical class="size-3" aria-hidden="true" />
		{m.demo_badge()}
		{#if !detailed}<span class="sr-only">{m.demo_explanation()}</span>{/if}
	</span>
	{#if detailed}<span class="text-xs text-muted-foreground">{m.demo_explanation()}</span>{/if}
</div>
