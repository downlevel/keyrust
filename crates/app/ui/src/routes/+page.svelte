<script lang="ts">
	import { invoke, isTauri } from "@tauri-apps/api/core";
	import KeyRound from "@lucide/svelte/icons/key-round";
	import { Button } from "$lib/components/ui/button/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Label } from "$lib/components/ui/label/index.js";
	import { m } from "$lib/paraglide/messages.js";

	type AppInfo = { version: string };

	let version = $state<string | null>(null);

	$effect(() => {
		if (!isTauri()) return;
		invoke<AppInfo>("app_info").then((info) => (version = info.version));
	});
</script>

<main class="flex min-h-screen items-center justify-center bg-zinc-50 p-6">
	<section class="w-full max-w-sm rounded-xl border bg-card p-8 shadow-sm">
		<header class="mb-6 flex items-center gap-3">
			<KeyRound class="size-7 text-zinc-900" aria-hidden="true" />
			<h1 class="text-2xl font-semibold tracking-tight">{m.app_name()}</h1>
		</header>

		<form class="space-y-4" onsubmit={(e) => e.preventDefault()}>
			<div class="space-y-2">
				<Label for="master-password">{m.unlock_master_password_label()}</Label>
				<Input
					id="master-password"
					type="password"
					autocomplete="off"
					spellcheck={false}
					disabled
				/>
			</div>
			<Button type="submit" class="w-full" disabled>{m.unlock_button()}</Button>
		</form>

		<p class="mt-6 text-sm text-muted-foreground">
			{m.prealpha_notice()}
		</p>
		<p class="mt-2 font-mono text-xs text-muted-foreground" data-testid="version">
			{version ? m.core_version({ version }) : m.core_version_unavailable()}
		</p>
	</section>
</main>
