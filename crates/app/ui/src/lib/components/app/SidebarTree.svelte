<script lang="ts">
	import Folder from "@lucide/svelte/icons/folder";
	import Layers from "@lucide/svelte/icons/layers";
	import Plus from "@lucide/svelte/icons/plus";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import { tick } from "svelte";
	import { ALL } from "$lib/state/app.svelte";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	let { collapsed = false }: { collapsed?: boolean } = $props();

	const app = useApp();

	interface Row {
		id: string;
		name: string;
		depth: number;
		count: number;
		kind: "all" | "group" | "bin";
	}

	const rows: Row[] = $derived([
		{ id: ALL, name: m.all_entries(), depth: 0, count: app.liveCount, kind: "all" as const },
		...app.groups
			.filter((g) => !g.isRecycleBin && !g.inRecycleBin)
			.map((g) => ({ ...g, kind: "group" as const })),
	]);
	const bin: Row | null = $derived.by(() => {
		const g = app.groups.find((x) => x.isRecycleBin);
		return g ? { ...g, name: m.recycle_bin(), kind: "bin" as const } : null;
	});
	const all = $derived(bin ? [...rows, bin] : rows);

	/**
	 * Position among siblings: the rows are flat in the DOM, so the tree
	 * structure is given by aria-level, aria-setsize and aria-posinset.
	 */
	const positions = $derived.by(() => {
		const parents = all.map((row, i) => {
			for (let j = i - 1; j >= 0; j--) if (all[j].depth < row.depth) return j;
			return -1;
		});
		const out: Record<string, { posinset: number; setsize: number }> = {};
		all.forEach((row, i) => {
			const siblings = all.filter(
				(r, j) => parents[j] === parents[i] && r.depth === row.depth,
			);
			out[row.id] = { posinset: siblings.indexOf(row) + 1, setsize: siblings.length };
		});
		return out;
	});

	const accessibleName = (row: Row) =>
		row.count === 1
			? m.group_with_count_one({ name: row.name })
			: m.group_with_count_many({ name: row.name, count: String(row.count) });

	let tree = $state<HTMLElement | null>(null);
	let newName = $state("");

	async function focusRow(id: string) {
		await tick();
		tree?.querySelector<HTMLElement>(`[data-group-id="${CSS.escape(id)}"]`)?.focus();
	}

	function onkeydown(event: KeyboardEvent, row: Row) {
		const index = all.findIndex((r) => r.id === row.id);
		let next: Row | undefined;
		if (event.key === "ArrowDown") next = all[index + 1];
		else if (event.key === "ArrowUp") next = all[index - 1];
		else if (event.key === "Home") next = all[0];
		else if (event.key === "End") next = all[all.length - 1];
		else return;
		event.preventDefault();
		if (next) {
			void app.selectGroup(next.id);
			void focusRow(next.id);
		}
	}

	function autofocus(node: HTMLInputElement) {
		node.focus();
		node.select();
	}
</script>

{#snippet item(row: Row)}
	{@const selected = app.groupId === row.id}
	{#if app.renamingGroupId === row.id}
		<input
			class="h-8 rounded-md border border-ring bg-background px-2 text-sm outline-none"
			style="margin-left: {row.depth * 16}px"
			aria-label={m.rename_group_label()}
			value={row.name}
			use:autofocus
			onkeydown={(e) => {
				if (e.key === "Enter") void app.renameGroup(row.id, e.currentTarget.value);
				if (e.key === "Escape") {
					e.stopPropagation();
					app.renamingGroupId = null;
				}
			}}
			onblur={(e) => void app.renameGroup(row.id, e.currentTarget.value)}
		/>
	{:else}
		<button
			type="button"
			role="treeitem"
			aria-selected={selected}
			aria-level={row.depth + 1}
			aria-posinset={positions[row.id]?.posinset}
			aria-setsize={positions[row.id]?.setsize}
			aria-label={accessibleName(row)}
			tabindex={selected ? 0 : -1}
			data-group-id={row.id}
			title={collapsed ? row.name : undefined}
			class="flex h-8 w-full items-center gap-2 rounded-md text-left {selected
				? 'bg-selection font-medium text-selection-foreground'
				: 'text-sidebar-foreground hover:bg-muted'} {collapsed
				? 'justify-center px-0'
				: 'px-2'}"
			style={collapsed ? undefined : `padding-left: ${8 + row.depth * 16}px`}
			onclick={() => void app.selectGroup(row.id)}
			onkeydown={(e) => onkeydown(e, row)}
			ondblclick={() => row.kind === "group" && app.startRenameGroup(row.id)}
		>
			{#if row.kind === "all"}<Layers class="size-4 shrink-0" aria-hidden="true" />
			{:else if row.kind === "bin"}<Trash2 class="size-4 shrink-0" aria-hidden="true" />
			{:else}<Folder class="size-4 shrink-0" aria-hidden="true" />{/if}
			{#if !collapsed}
				<span class="min-w-0 flex-1 truncate">{row.name}</span>
				<span class="text-xs font-normal text-muted-foreground">{row.count}</span>
			{/if}
		</button>
	{/if}
{/snippet}

<div
	bind:this={tree}
	role="tree"
	aria-label={m.sidebar_groups()}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="flex flex-col gap-px overflow-y-auto">
		{#each rows as row (row.id)}
			{@render item(row)}
		{/each}
		{#if app.renamingGroupId === "new"}
			<input
				class="h-8 rounded-md border border-ring bg-background px-2 text-sm outline-none"
				aria-label={m.new_group()}
				placeholder={m.new_group_placeholder()}
				bind:value={newName}
				use:autofocus
				onkeydown={(e) => {
					if (e.key === "Enter") {
						app.renamingGroupId = null;
						void app.createGroup(newName);
						newName = "";
					}
					if (e.key === "Escape") {
						e.stopPropagation();
						app.renamingGroupId = null;
						newName = "";
					}
				}}
				onblur={() => {
					if (app.renamingGroupId === "new") app.renamingGroupId = null;
					newName = "";
				}}
			/>
		{/if}
	</div>
	<div class="flex-1"></div>
	{#if bin}{@render item(bin)}{/if}
</div>
{#if !collapsed}
	<button
		type="button"
		class="mt-px flex h-8 items-center gap-2 rounded-md px-2 text-left text-muted-foreground hover:bg-muted hover:text-foreground"
		onclick={() => void app.run("new_group")}
	>
		<Plus class="size-4" aria-hidden="true" />
		{m.new_group()}
	</button>
{/if}
