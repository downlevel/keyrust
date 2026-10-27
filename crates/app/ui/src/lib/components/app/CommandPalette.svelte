<script lang="ts">
	import FileText from "@lucide/svelte/icons/file-text";
	import * as Command from "$lib/components/ui/command/index.js";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { COMMANDS, commandLabel, formatShortcut, type CommandId } from "$lib/commands/commands";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();
	const open = $derived(app.overlay === "palette");

	const VAULT_ONLY = new Set<CommandId>([
		"save",
		"lock",
		"close_database",
		"find",
		"new_entry",
		"edit_entry",
		"copy_username",
		"copy_password",
		"copy_url",
		"delete_entry",
		"new_group",
		"rename_group",
		"delete_group",
		"database_settings",
		"demo_external_change",
	]);

	const commands = $derived(
		COMMANDS.filter(
			(c) =>
				c.id !== "command_palette" &&
				(!c.demoOnly || app.isDemo) &&
				(app.screen === "vault" || !VAULT_ONLY.has(c.id)),
		),
	);
	const entries = $derived(
		app.screen === "vault"
			? app.entries
					.filter((e) => !app.isRecycled(e.groupId))
					.sort((a, b) => a.title.localeCompare(b.title))
			: [],
	);

	function close() {
		if (app.overlay === "palette") app.closeOverlay();
	}

	function runCommand(id: CommandId) {
		close();
		// Let the dialog release focus before the command moves it.
		queueMicrotask(() => void app.run(id));
	}

	function openEntry(id: string) {
		close();
		queueMicrotask(() => {
			void app.requestLeave(async () => {
				if (app.groupId !== "all") await app.selectGroup("all");
				app.setQuery("");
				await app.selectEntry(id);
			});
		});
	}
</script>

<Dialog.Root {open} onOpenChange={(o) => !o && close()}>
	<Dialog.Content
		class="top-1/4 translate-y-0 overflow-hidden rounded-xl p-0 sm:max-w-[560px]"
		showCloseButton={false}
	>
		<Dialog.Title class="sr-only">{m.palette_label()}</Dialog.Title>
		<Command.Root>
			<Command.Input placeholder={m.palette_placeholder()} aria-label={m.palette_label()} />
			<Command.List class="max-h-[360px]">
				<Command.Empty>{m.palette_empty()}</Command.Empty>
				<Command.Group heading={m.palette_commands()}>
					{#each commands as c (c.id)}
						<Command.Item
							value={`cmd:${c.id}`}
							keywords={[commandLabel(c)]}
							onSelect={() => runCommand(c.id)}
						>
							<span class="flex-1">{commandLabel(c)}</span>
							{#if c.shortcut}<Command.Shortcut
									>{formatShortcut(c.shortcut)}</Command.Shortcut
								>{/if}
						</Command.Item>
					{/each}
				</Command.Group>
				{#if entries.length}
					<Command.Group heading={m.palette_entries()}>
						{#each entries as e (e.id)}
							<Command.Item
								value={`entry:${e.id}`}
								keywords={[e.title, e.username]}
								onSelect={() => openEntry(e.id)}
							>
								<FileText class="size-4 text-muted-foreground" aria-hidden="true" />
								<span class="flex-1 truncate">{e.title}</span>
								<span class="truncate text-xs text-muted-foreground"
									>{e.username}</span
								>
							</Command.Item>
						{/each}
					</Command.Group>
				{/if}
			</Command.List>
		</Command.Root>
	</Dialog.Content>
</Dialog.Root>
