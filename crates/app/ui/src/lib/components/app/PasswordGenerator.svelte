<script lang="ts">
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import type { StrengthEstimate } from "$lib/api";
	import { estimateStrength } from "$lib/api/strength";
	import { Button } from "$lib/components/ui/button/index.js";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";
	import StrengthMeter from "./StrengthMeter.svelte";

	const app = useApp();

	const options = [
		["lowercase", m.generator_lowercase],
		["uppercase", m.generator_uppercase],
		["digits", m.generator_digits],
		["symbols", m.generator_symbols],
		["excludeAmbiguous", m.generator_exclude_ambiguous],
	] as const;

	let strength = $state<StrengthEstimate>(estimateStrength(""));
	let sequence = 0;
	$effect(() => {
		const value = app.generated;
		const current = ++sequence;
		void app.api.vault.estimateStrength(value).then((s) => {
			if (current === sequence) strength = s;
		});
	});

	const canUse = $derived(!!app.editor && !!app.generated);
</script>

<div class="flex flex-col gap-3.5" data-testid="password-generator">
	<div class="flex items-center gap-2 rounded-lg border border-border bg-surface px-3 py-2.5">
		<output
			class="min-w-0 flex-1 font-mono text-base break-all"
			aria-label={m.generator_title()}
			data-testid="generated-password">{app.generated}</output
		>
		<button
			type="button"
			class="flex size-8 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
			aria-label={m.generator_regenerate()}
			title={m.generator_regenerate()}
			onclick={() => void app.regenerate()}
		>
			<RefreshCw class="size-4" aria-hidden="true" />
		</button>
	</div>
	<div class="flex flex-col gap-1.5">
		<div class="flex justify-between">
			<label for="generator-length" class="text-[13px] font-medium"
				>{m.generator_length()}</label
			>
			<span class="font-mono text-[13px]">{app.generator.length}</span>
		</div>
		<input
			id="generator-length"
			type="range"
			min="8"
			max="128"
			value={app.generator.length}
			class="w-full accent-primary"
			oninput={(e) => app.setGenerator({ length: Number(e.currentTarget.value) })}
		/>
	</div>
	<div class="grid grid-cols-2 gap-x-4 gap-y-2">
		{#each options as [key, label] (key)}
			<label class="flex cursor-pointer items-center gap-2 text-[13px]">
				<input
					type="checkbox"
					class="size-4 accent-primary"
					checked={app.generator[key]}
					onchange={(e) => app.setGenerator({ [key]: e.currentTarget.checked })}
				/>
				{label()}
			</label>
		{/each}
	</div>
	<StrengthMeter estimate={strength} />
	{#if app.editor}
		<Button disabled={!canUse} onclick={() => app.useGenerated()}>{m.generator_use()}</Button>
	{:else}
		<p class="text-xs text-muted-foreground">{m.generator_copy_hint()}</p>
	{/if}
</div>
