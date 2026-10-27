<script lang="ts">
	import Lock from "@lucide/svelte/icons/lock";
	import PanelLeft from "@lucide/svelte/icons/panel-left";
	import Plus from "@lucide/svelte/icons/plus";
	import Search from "@lucide/svelte/icons/search";
	import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
	import { tick } from "svelte";
	import { Button } from "$lib/components/ui/button/index.js";
	import Banner from "$lib/components/app/Banner.svelte";
	import DemoBadge from "$lib/components/app/DemoBadge.svelte";
	import EntryDetail from "$lib/components/app/EntryDetail.svelte";
	import EntryEditor from "$lib/components/app/EntryEditor.svelte";
	import EntryListItem from "$lib/components/app/EntryListItem.svelte";
	import SearchInput from "$lib/components/app/SearchInput.svelte";
	import SidebarTree from "$lib/components/app/SidebarTree.svelte";
	import UnsavedDot from "$lib/components/app/UnsavedDot.svelte";
	import Wordmark from "$lib/components/app/Wordmark.svelte";
	import { formatShortcut } from "$lib/commands/commands";
	import { ALL } from "$lib/state/app.svelte";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();

	let width = $state(globalThis.innerWidth ?? 1280);
	let expanded = $state(false);
	const narrow = $derived(width < 1100);
	const collapsed = $derived(narrow && !expanded);

	let list = $state<HTMLElement | null>(null);

	const caption = $derived(
		app.visibleEntries.length === 1
			? m.entry_count_one()
			: m.entry_count_many({ count: String(app.visibleEntries.length) }),
	);
	const emptyText = $derived(
		app.query.trim()
			? null
			: app.groupId === ALL && app.liveCount === 0
				? "vault"
				: app.groupId === app.recycleBinId
					? m.empty_recycle_bin()
					: m.empty_group(),
	);
	const status = $derived(
		app.saveState === "saving"
			? m.status_saving()
			: app.saveState === "failed"
				? m.status_failed()
				: m.status_saved(),
	);

	async function focusSelected() {
		await tick();
		list?.querySelector<HTMLElement>('[aria-selected="true"]')?.focus();
	}

	function onEntryKeydown(event: KeyboardEvent, id: string) {
		if (event.key === "ArrowDown" || event.key === "ArrowUp") {
			event.preventDefault();
			app.moveSelection(event.key === "ArrowDown" ? 1 : -1);
			void focusSelected();
		} else if (event.key === "Enter" && !event.ctrlKey && !event.metaKey) {
			event.preventDefault();
			if (app.selectedId !== id) void app.selectEntry(id);
			else void app.editEntry(id);
		}
	}
</script>

<svelte:window bind:innerWidth={width} />

<div class="flex min-w-0 flex-1">
	<!-- Groups -->
	<nav
		aria-label={m.sidebar_groups()}
		class="flex shrink-0 flex-col border-r border-border bg-sidebar px-2 py-3 {collapsed
			? 'w-14'
			: 'w-[220px]'}"
	>
		<div class="flex items-center gap-2 px-2 pt-1 pb-3 {collapsed ? 'flex-col px-0' : ''}">
			<Wordmark size="sm" />
			{#if !collapsed}
				<span class="min-w-0 flex-1 truncate font-semibold" data-testid="db-name"
					>{app.db?.name}</span
				>
			{/if}
			{#if app.editorDirty}<UnsavedDot />{/if}
			<button
				type="button"
				class="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
				aria-label={m.lock_button()}
				title={`${m.lock_button()} (${formatShortcut("CmdOrCtrl+L")})`}
				onclick={() => app.lock()}
			>
				<Lock class="size-4" aria-hidden="true" />
			</button>
		</div>
		{#if narrow}
			<button
				type="button"
				class="mb-1 flex h-8 items-center gap-2 rounded-md px-2 text-muted-foreground hover:bg-muted hover:text-foreground {collapsed
					? 'justify-center'
					: ''}"
				aria-label={collapsed ? m.expand_sidebar() : m.collapse_sidebar()}
				aria-expanded={!collapsed}
				onclick={() => (expanded = !expanded)}
			>
				<PanelLeft class="size-4" aria-hidden="true" />
			</button>
		{/if}
		<SidebarTree {collapsed} />
		<div class="mx-1 my-2 h-px bg-border"></div>
		<div class="flex items-center gap-1 px-1 {collapsed ? 'flex-col' : ''}">
			<button
				type="button"
				class="flex h-8 items-center gap-2 rounded-md px-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
				aria-label={collapsed ? m.settings_button() : undefined}
				title={`${m.settings_button()} (${formatShortcut("CmdOrCtrl+Comma")})`}
				onclick={() => app.openSettings()}
			>
				<SlidersHorizontal class="size-4" aria-hidden="true" />
				{#if !collapsed}{m.settings_button()}{/if}
			</button>
			{#if !collapsed}
				<span
					class="ml-auto pr-1 text-xs text-muted-foreground"
					role="status"
					data-testid="save-status">{status}</span
				>
			{/if}
		</div>
	</nav>

	<!-- Entries -->
	<section
		aria-label={m.entries_label()}
		class="flex w-[320px] shrink-0 flex-col border-r border-border bg-background"
	>
		<div class="flex flex-col gap-2.5 border-b border-muted p-3">
			<SearchInput
				value={app.query}
				focusSignal={app.focusSearch}
				oninput={(q) => app.setQuery(q)}
				onarrow={() => {
					if (!app.selectedId) app.moveSelection(0);
					void focusSelected();
				}}
			/>
			<div class="flex items-center justify-between">
				<span class="text-xs text-muted-foreground" aria-live="polite">{caption}</span>
				<button
					type="button"
					class="flex h-7 items-center gap-1.5 rounded-md px-2 text-[13px] font-medium text-primary hover:bg-muted"
					title={`${m.new_entry()} (${formatShortcut("CmdOrCtrl+N")})`}
					onclick={() => void app.newEntry()}
				>
					<Plus class="size-3.5" aria-hidden="true" strokeWidth={2.4} />
					{m.new_entry()}
				</button>
			</div>
		</div>
		<div
			bind:this={list}
			role="listbox"
			aria-label={m.entries_label()}
			class="flex-1 overflow-y-auto p-1.5"
		>
			{#each app.visibleEntries as entry (entry.id)}
				<EntryListItem
					{entry}
					selected={entry.id === app.selectedId && !app.editor?.isNew}
					onselect={() => void app.selectEntry(entry.id)}
					onopen={() => void app.editEntry(entry.id)}
					onkeydown={(e) => onEntryKeydown(e, entry.id)}
				/>
			{/each}
			{#if app.visibleEntries.length === 0}
				{#if app.query.trim()}
					<div
						class="flex flex-col items-center gap-2 px-4 py-10 text-center text-muted-foreground"
					>
						<Search class="size-5" aria-hidden="true" />
						<span class="font-medium text-foreground"
							>{m.no_matches({ query: app.query.trim() })}</span
						>
						<span class="text-xs">{m.no_matches_hint()}</span>
					</div>
				{:else if emptyText === "vault"}
					<div class="flex flex-col items-center gap-3 px-4 py-10 text-center">
						<div
							class="flex size-11 items-center justify-center rounded-xl bg-selection text-primary"
						>
							<Plus class="size-5" aria-hidden="true" />
						</div>
						<span class="font-semibold">{m.empty_vault_title()}</span>
						<span class="text-muted-foreground"
							>{m.empty_vault_body({ shortcut: formatShortcut("CmdOrCtrl+N") })}</span
						>
						<Button onclick={() => void app.newEntry()}>{m.new_entry()}</Button>
					</div>
				{:else}
					<p class="px-4 py-10 text-center text-muted-foreground">{emptyText}</p>
				{/if}
			{/if}
		</div>
	</section>

	<!-- Detail -->
	<main class="flex min-w-0 flex-1 flex-col bg-background">
		{#if app.banner === "externalChange"}
			<Banner tone="warning" message={m.banner_external()}>
				{#snippet actions()}
					<Button size="sm" variant="outline" onclick={() => void app.keepMine()}
						>{m.banner_keep_mine()}</Button
					>
					<Button
						size="sm"
						class="bg-warning-strong text-primary-foreground hover:bg-warning-strong/90"
						onclick={() => void app.reloadExternal()}>{m.banner_reload()}</Button
					>
				{/snippet}
			</Banner>
		{:else if app.banner === "saveFailed"}
			<Banner tone="danger" message={m.banner_save_failed()}>
				{#snippet actions()}
					<Button size="sm" variant="outline" onclick={() => void app.persist()}
						>{m.banner_save_as()}</Button
					>
					<Button
						size="sm"
						class="bg-destructive text-destructive-foreground hover:bg-destructive-hover"
						onclick={() => void app.persist()}>{m.banner_retry()}</Button
					>
				{/snippet}
			</Banner>
		{/if}
		{#if app.isDemo}
			<div class="flex justify-end px-6 pt-3"><DemoBadge /></div>
		{/if}
		<div class="min-h-0 flex-1 overflow-y-auto">
			{#if app.editor}
				<EntryEditor />
			{:else}
				<EntryDetail />
			{/if}
		</div>
	</main>
</div>
