<script lang="ts">
	import { onMount } from "svelte";
	import { VaultError } from "$lib/api";
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import Check from "@lucide/svelte/icons/check";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";
	import type { Cipher, StrengthEstimate } from "$lib/api";
	import { clampKdf, KDF_LIMITS } from "$lib/api/app";
	import { estimateStrength } from "$lib/api/strength";
	import { Button } from "$lib/components/ui/button/index.js";
	import DemoBadge from "$lib/components/app/DemoBadge.svelte";
	import SecretInput from "$lib/components/app/SecretInput.svelte";
	import StrengthMeter from "$lib/components/app/StrengthMeter.svelte";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();

	let path = $state("");
	let name = $state("Personal");
	let password = $state("");
	let confirm = $state("");
	let advanced = $state(false);
	let cipher = $state<Cipher>("aes256");
	let memoryMib = $state(64);
	let iterations = $state(3);
	let parallelism = $state(2);
	let benchmarked = $state(false);

	let strength = $state<StrengthEstimate>(estimateStrength(""));
	let sequence = 0;
	$effect(() => {
		const value = password;
		const current = ++sequence;
		void app.api.vault.estimateStrength(value).then((s) => {
			if (current === sequence) strength = s;
		});
	});

	const creating = $derived(app.busy === "create");
	const matches = $derived(confirm.length > 0 && confirm === password);
	const canSubmit = $derived(!!path && !!name.trim() && !!password && matches && !creating);

	// New databases get Argon2id iterations measured on this computer (~1 s).
	onMount(() => {
		void benchmark();
	});

	async function choose() {
		try {
			path = (await app.api.vault.chooseNewPath(name)).path;
		} catch (e) {
			if (!(e instanceof VaultError && e.kind === "cancelled")) app.createError = true;
		}
	}

	async function benchmark() {
		const result = await app.benchmark(memoryMib, parallelism);
		if (result !== null) {
			iterations = result;
			benchmarked = true;
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!canSubmit) return;
		const secret = password;
		password = "";
		confirm = "";
		await app.create({
			path,
			name,
			password: secret,
			cipher,
			kdf: clampKdf({ memoryMib, iterations, parallelism }),
		});
	}

	const input =
		"h-[38px] rounded-lg border border-input bg-background px-3 outline-none focus:border-ring focus:ring-3 focus:ring-ring/20";
	const small =
		"h-[34px] rounded-lg border border-input bg-background px-2 outline-none focus:border-ring focus:ring-3 focus:ring-ring/20";
</script>

<main class="flex flex-1 items-start justify-center overflow-y-auto p-8">
	<form
		class="my-auto flex w-[520px] flex-col rounded-xl border border-border bg-card shadow-xs"
		onsubmit={submit}
		novalidate
	>
		<header class="flex flex-col gap-1.5 px-7 pt-6">
			<div class="flex items-center justify-between">
				<button
					type="button"
					class="text-[13px] font-medium text-primary hover:text-primary-hover"
					onclick={() => app.cancelCreate()}
					><ArrowLeft class="inline size-3.5" aria-hidden="true" />
					{m.create_back()}</button
				>
				{#if app.isDemo}<DemoBadge />{/if}
			</div>
			<h1 class="text-xl font-semibold tracking-tight">{m.create_title()}</h1>
			<p class="text-muted-foreground">{m.create_subtitle()}</p>
		</header>

		<div class="flex flex-col gap-4 px-7 py-5">
			<div class="flex flex-col gap-1.5">
				<label for="create-file" class="text-[13px] font-medium"
					>{m.create_file_label()}</label
				>
				<div class="flex gap-2">
					<input
						id="create-file"
						class="{input} min-w-0 flex-1 bg-surface text-foreground/80"
						value={path}
						placeholder={m.create_no_file()}
						readonly
					/>
					<Button type="button" variant="outline" class="h-[38px]" onclick={choose}
						>{m.create_choose()}</Button
					>
				</div>
			</div>
			<div class="flex flex-col gap-1.5">
				<label for="create-name" class="text-[13px] font-medium"
					>{m.create_name_label()}</label
				>
				<input id="create-name" class={input} bind:value={name} />
			</div>
			<div class="flex flex-col gap-1.5">
				<label for="create-password" class="text-[13px] font-medium"
					>{m.create_password_label()}</label
				>
				<SecretInput
					id="create-password"
					bind:value={password}
					autocomplete="new-password"
					describedby="create-password-hint"
				/>
				<StrengthMeter estimate={strength} />
				<span id="create-password-hint" class="text-xs text-muted-foreground"
					>{m.create_password_hint()}</span
				>
			</div>
			<div class="flex flex-col gap-1.5">
				<label for="create-confirm" class="text-[13px] font-medium"
					>{m.create_confirm_label()}</label
				>
				<SecretInput
					id="create-confirm"
					bind:value={confirm}
					autocomplete="new-password"
					invalid={confirm.length > 0 && !matches}
					describedby="create-confirm-status"
					showCapsWarning={false}
				/>
				<span id="create-confirm-status" class="min-h-4 text-xs" aria-live="polite">
					{#if matches}
						<span class="inline-flex items-center gap-1.5 text-success-text">
							<Check class="size-3.5" aria-hidden="true" />{m.create_match()}
						</span>
					{:else if confirm.length > 0}
						<span class="text-destructive">{m.create_mismatch()}</span>
					{/if}
				</span>
			</div>

			<div class="rounded-[10px] border border-border">
				<button
					type="button"
					class="flex h-[42px] w-full items-center justify-between rounded-[10px] px-3.5 font-medium hover:bg-surface"
					aria-expanded={advanced}
					aria-controls="create-advanced"
					onclick={() => (advanced = !advanced)}
				>
					{m.create_advanced()}
					<ChevronDown
						class="size-4 text-muted-foreground transition-transform {advanced
							? 'rotate-180'
							: ''}"
						aria-hidden="true"
					/>
				</button>
				{#if advanced}
					<div id="create-advanced" class="flex flex-col gap-3 px-3.5 pb-3.5">
						<div class="grid grid-cols-2 gap-3">
							<label class="flex flex-col gap-1 text-[13px]"
								>{m.cipher_label()}
								<select class={small} bind:value={cipher}>
									<option value="aes256">{m.cipher_aes()}</option>
									<option value="chaCha20">{m.cipher_chacha()}</option>
								</select>
							</label>
							<label class="flex flex-col gap-1 text-[13px]"
								>{m.kdf_label()}
								<select class={small} disabled
									><option>{m.kdf_argon2id()}</option></select
								>
							</label>
						</div>
						<div class="grid grid-cols-3 gap-3">
							<label class="flex flex-col gap-1 text-[13px]"
								>{m.kdf_memory()}<input
									type="number"
									min={KDF_LIMITS.memoryMib.min}
									max={KDF_LIMITS.memoryMib.max}
									class="{small} font-mono"
									bind:value={memoryMib}
								/></label
							>
							<label class="flex flex-col gap-1 text-[13px]"
								>{m.kdf_iterations()}<input
									type="number"
									min={KDF_LIMITS.iterations.min}
									max={KDF_LIMITS.iterations.max}
									class="{small} font-mono"
									bind:value={iterations}
								/></label
							>
							<label class="flex flex-col gap-1 text-[13px]"
								>{m.kdf_parallelism()}<input
									type="number"
									min={KDF_LIMITS.parallelism.min}
									max={KDF_LIMITS.parallelism.max}
									class="{small} font-mono"
									bind:value={parallelism}
								/></label
							>
						</div>
						<div class="flex items-center gap-3">
							<Button
								type="button"
								variant="outline"
								disabled={app.busy === "benchmark"}
								onclick={benchmark}
							>
								{app.busy === "benchmark"
									? m.kdf_benchmark_busy()
									: m.kdf_benchmark()}
							</Button>
							{#if benchmarked}
								<span class="text-xs text-muted-foreground" role="status"
									>{m.kdf_benchmark_result({
										iterations: String(iterations),
									})}</span
								>
							{/if}
						</div>
					</div>
				{/if}
			</div>
			{#if app.createError}
				<p class="text-[13px] text-destructive" role="alert">{m.create_error()}</p>
			{/if}
		</div>

		<footer class="flex justify-end gap-2 border-t border-muted px-7 py-4">
			<Button type="button" variant="outline" onclick={() => app.cancelCreate()}
				>{m.create_cancel()}</Button
			>
			<Button type="submit" disabled={!canSubmit}>
				{#if creating}<LoaderCircle
						class="size-4 animate-spin"
						aria-hidden="true"
					/>{m.create_busy()}{:else}{m.create_submit()}{/if}
			</Button>
		</footer>
	</form>
</main>
