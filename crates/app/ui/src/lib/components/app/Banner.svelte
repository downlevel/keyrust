<script lang="ts">
	import CircleAlert from "@lucide/svelte/icons/circle-alert";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import type { Snippet } from "svelte";

	let {
		tone,
		message,
		actions,
	}: { tone: "warning" | "danger"; message: string; actions?: Snippet } = $props();
</script>

<div
	role="alert"
	class="flex items-center gap-3 border-b px-4 py-2.5 {tone === 'warning'
		? 'border-warning-soft-border bg-warning-soft text-warning-soft-foreground'
		: 'border-destructive-soft-border bg-destructive-soft text-destructive-soft-foreground'}"
>
	{#if tone === "warning"}
		<TriangleAlert class="size-4 shrink-0 text-warning-strong" aria-hidden="true" />
	{:else}
		<CircleAlert class="size-4 shrink-0 text-destructive" aria-hidden="true" />
	{/if}
	<span class="flex-1">{message}</span>
	{#if actions}<div class="flex shrink-0 gap-2">{@render actions()}</div>{/if}
</div>
