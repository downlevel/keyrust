<script lang="ts">
	import { invoke, isTauri } from "@tauri-apps/api/core";
	import KeyRound from "@lucide/svelte/icons/key-round";
	import { Button } from "$lib/components/ui/button/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Label } from "$lib/components/ui/label/index.js";

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
			<h1 class="text-2xl font-semibold tracking-tight">Keyrust</h1>
		</header>

		<form class="space-y-4" onsubmit={(e) => e.preventDefault()}>
			<div class="space-y-2">
				<Label for="master-password">Master password</Label>
				<Input
					id="master-password"
					type="password"
					autocomplete="off"
					spellcheck={false}
					disabled
				/>
			</div>
			<Button type="submit" class="w-full" disabled>Unlock</Button>
		</form>

		<p class="mt-6 text-sm text-muted-foreground">
			Pre-alpha: database support coming in Milestone 3.
		</p>
		<p class="mt-2 font-mono text-xs text-muted-foreground" data-testid="version">
			{version ? `core v${version}` : "core version unavailable outside Tauri"}
		</p>
	</section>
</main>
