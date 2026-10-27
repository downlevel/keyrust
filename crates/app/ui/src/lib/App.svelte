<script lang="ts">
	import { isTauri } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import type { Apis } from "$lib/api";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import CommandPalette from "$lib/components/app/CommandPalette.svelte";
	import ConfirmDialog from "$lib/components/app/ConfirmDialog.svelte";
	import PasswordGenerator from "$lib/components/app/PasswordGenerator.svelte";
	import SettingsDialog from "$lib/components/app/SettingsDialog.svelte";
	import ShortcutsPanel from "$lib/components/app/ShortcutsPanel.svelte";
	import Toast from "$lib/components/app/Toast.svelte";
	import { isCommandId } from "$lib/commands/commands";
	import CreateScreen from "$lib/screens/CreateScreen.svelte";
	import UnlockScreen from "$lib/screens/UnlockScreen.svelte";
	import VaultScreen from "$lib/screens/VaultScreen.svelte";
	import { AppState } from "$lib/state/app.svelte";
	import { provideApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	let { apis }: { apis: Apis } = $props();

	// The state lives as long as the component; `apis` never changes.
	// svelte-ignore state_referenced_locally
	const app = provideApp(new AppState(apis));

	onMount(() => {
		void app.init();
		const onCopy = (event: ClipboardEvent) => app.handleCopy(event);
		document.addEventListener("copy", onCopy);
		let unlisten: (() => void) | undefined;
		if (isTauri()) {
			// Native menu clicks arrive as `menu` events carrying the command id.
			void import("@tauri-apps/api/event").then(({ listen }) =>
				listen<string>("menu", (event) => {
					if (isCommandId(event.payload)) void app.run(event.payload);
				}).then((stop) => (unlisten = stop)),
			);
		}
		return () => {
			document.removeEventListener("copy", onCopy);
			unlisten?.();
		};
	});

	const standaloneGenerator = $derived(app.overlay === "generator" && !app.editor);
</script>

<svelte:window onkeydown={(e) => app.handleKeydown(e)} />

<div class="flex h-screen min-h-[560px] min-w-[800px] bg-surface text-foreground">
	{#if app.screen === "vault"}
		<VaultScreen />
	{:else if app.screen === "create"}
		<CreateScreen />
	{:else}
		<UnlockScreen />
	{/if}
</div>

<SettingsDialog />
<ShortcutsPanel />
<CommandPalette />
<ConfirmDialog />
<Toast />

<Dialog.Root open={standaloneGenerator} onOpenChange={(o) => !o && app.closeOverlay()}>
	<Dialog.Content class="sm:max-w-[420px]">
		<Dialog.Header>
			<Dialog.Title>{m.generator_title()}</Dialog.Title>
		</Dialog.Header>
		<PasswordGenerator />
	</Dialog.Content>
</Dialog.Root>
