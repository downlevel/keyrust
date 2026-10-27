<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import {
		COMMANDS,
		MENUS,
		commandLabel,
		formatShortcut,
		menuLabel,
	} from "$lib/commands/commands";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();
	const open = $derived(app.overlay === "shortcuts");

	const sections = MENUS.map((menu) => ({
		id: menu.id,
		title: menuLabel(menu.labelKey),
		items: COMMANDS.filter((c) => c.menu === menu.id && c.shortcut).map((c) => ({
			id: c.id,
			label: commandLabel(c),
			keys: formatShortcut(c.shortcut as string),
		})),
	})).filter((s) => s.items.length);

	const navigation = [
		{ id: "arrows", label: m.shortcut_arrows, keys: "↑ ↓" },
		{ id: "enter", label: m.shortcut_enter, keys: "Enter" },
		{ id: "escape", label: m.shortcut_escape, keys: "Esc" },
	];
</script>

<Dialog.Root {open} onOpenChange={(o) => !o && app.closeOverlay()}>
	<Dialog.Content class="max-h-[80vh] overflow-y-auto sm:max-w-[640px]">
		<Dialog.Header>
			<Dialog.Title>{m.shortcuts_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="grid grid-cols-1 gap-x-8 gap-y-5 sm:grid-cols-2">
			{#each sections as section (section.id)}
				<section class="flex flex-col gap-1.5">
					<h3
						class="text-xs font-semibold tracking-wider text-muted-foreground uppercase"
					>
						{section.title}
					</h3>
					<dl class="flex flex-col">
						{#each section.items as item (item.id)}
							<div class="flex items-center justify-between gap-4 py-1">
								<dt>{item.label}</dt>
								<dd>
									<kbd
										class="rounded border border-b-2 border-border bg-surface px-1.5 font-sans text-xs"
										>{item.keys}</kbd
									>
								</dd>
							</div>
						{/each}
					</dl>
				</section>
			{/each}
			<section class="flex flex-col gap-1.5">
				<h3 class="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
					{m.shortcuts_navigation()}
				</h3>
				<dl class="flex flex-col">
					{#each navigation as item (item.id)}
						<div class="flex items-center justify-between gap-4 py-1">
							<dt>{item.label()}</dt>
							<dd>
								<kbd
									class="rounded border border-b-2 border-border bg-surface px-1.5 font-sans text-xs"
									>{item.keys}</kbd
								>
							</dd>
						</div>
					{/each}
				</dl>
			</section>
		</div>
	</Dialog.Content>
</Dialog.Root>
