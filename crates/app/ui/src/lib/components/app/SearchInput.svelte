<script lang="ts">
	import Search from "@lucide/svelte/icons/search";
	import X from "@lucide/svelte/icons/x";
	import { formatShortcut } from "$lib/commands/commands";
	import { m } from "$lib/paraglide/messages.js";

	let {
		value,
		focusSignal = 0,
		oninput,
		onarrow,
	}: {
		value: string;
		focusSignal?: number;
		oninput: (value: string) => void;
		/** ArrowDown from the field moves into the list. */
		onarrow?: () => void;
	} = $props();

	let input = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (focusSignal > 0) {
			input?.focus();
			input?.select();
		}
	});
</script>

<div
	class="flex h-9 items-center gap-2 rounded-lg border border-border bg-surface pr-2 pl-2.5 focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/20"
>
	<Search class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
	<input
		bind:this={input}
		type="search"
		class="h-8 min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted-foreground [&::-webkit-search-cancel-button]:hidden"
		aria-label={m.search_label()}
		placeholder={m.search_placeholder()}
		{value}
		oninput={(e) => oninput(e.currentTarget.value)}
		onkeydown={(e) => {
			if (e.key === "ArrowDown" && onarrow) {
				e.preventDefault();
				onarrow();
			} else if (e.key === "Escape" && value) {
				e.preventDefault();
				e.stopPropagation();
				oninput("");
			}
		}}
	/>
	{#if value}
		<button
			type="button"
			class="flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
			aria-label={m.search_clear()}
			onclick={() => {
				oninput("");
				input?.focus();
			}}
		>
			<X class="size-3.5" aria-hidden="true" />
		</button>
	{:else}
		<kbd
			class="rounded border border-b-2 border-border bg-background px-1.5 font-sans text-[11px] text-muted-foreground"
			>{formatShortcut("CmdOrCtrl+F")}</kbd
		>
	{/if}
</div>
