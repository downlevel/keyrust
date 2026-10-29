<script lang="ts">
	import CircleAlert from "@lucide/svelte/icons/circle-alert";
	import Info from "@lucide/svelte/icons/info";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import type { Snippet } from "svelte";

	let {
		tone,
		message,
		actions,
	}: { tone: "info" | "warning" | "danger"; message: string; actions?: Snippet } = $props();

	const toneClass = {
		info: "border-border bg-surface text-foreground",
		warning: "border-warning-soft-border bg-warning-soft text-warning-soft-foreground",
		danger: "border-destructive-soft-border bg-destructive-soft text-destructive-soft-foreground",
	};
</script>

<!-- Information is announced politely; problems interrupt. -->
<div
	role={tone === "info" ? "status" : "alert"}
	class="flex items-center gap-3 border-b px-4 py-2.5 {toneClass[tone]}"
>
	{#if tone === "info"}
		<Info class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
	{:else if tone === "warning"}
		<TriangleAlert class="size-4 shrink-0 text-warning-strong" aria-hidden="true" />
	{:else}
		<CircleAlert class="size-4 shrink-0 text-destructive" aria-hidden="true" />
	{/if}
	<span class="flex-1">{message}</span>
	{#if actions}<div class="flex shrink-0 gap-2">{@render actions()}</div>{/if}
</div>
