<script lang="ts">
	import type { EntrySummary } from "$lib/api";
	import { avatarIndex, domain, initial } from "./avatar";

	let {
		entry,
		selected,
		tabbable = selected,
		onselect,
		onopen,
		onkeydown,
	}: {
		entry: EntrySummary;
		selected: boolean;
		/** In the tab order (the list's single tab stop). */
		tabbable?: boolean;
		onselect: () => void;
		onopen: () => void;
		onkeydown: (event: KeyboardEvent) => void;
	} = $props();

	const tone = $derived(avatarIndex(entry.title));
</script>

<button
	type="button"
	role="option"
	aria-selected={selected}
	tabindex={tabbable ? 0 : -1}
	data-entry-id={entry.id}
	class="relative flex h-[52px] w-full items-center gap-3 rounded-lg px-2.5 text-left {selected
		? 'bg-selection'
		: 'hover:bg-muted'}"
	onclick={onselect}
	ondblclick={onopen}
	{onkeydown}
>
	{#if selected}
		<span class="absolute inset-y-2.5 left-0 w-[3px] rounded-full bg-primary" aria-hidden="true"
		></span>
	{/if}
	<span
		class="flex size-8 shrink-0 items-center justify-center rounded-lg text-[13px] font-semibold"
		style="background: var(--avatar-{tone}-bg); color: var(--avatar-{tone}-fg)"
		aria-hidden="true">{initial(entry.title)}</span
	>
	<span class="flex min-w-0 flex-1 flex-col">
		<span class="truncate font-medium">{entry.title}</span>
		<span class="truncate text-xs text-muted-foreground">{entry.username}</span>
	</span>
	<span class="max-w-24 truncate text-xs text-muted-foreground">{domain(entry.url)}</span>
</button>
