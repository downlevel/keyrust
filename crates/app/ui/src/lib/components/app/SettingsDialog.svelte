<script lang="ts">
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { Switch } from "$lib/components/ui/switch/index.js";
	import type { Argon2idParams, Cipher, StrengthEstimate } from "$lib/api";
	import { estimateStrength } from "$lib/api/strength";
	import SecretInput from "./SecretInput.svelte";
	import StrengthMeter from "./StrengthMeter.svelte";
	import type { SettingsTab } from "$lib/state/app.svelte";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";
	import Wordmark from "./Wordmark.svelte";

	const app = useApp();
	const open = $derived(app.overlay === "settings");

	const tabs: { id: SettingsTab; label: () => string }[] = [
		{ id: "general", label: m.settings_general },
		{ id: "database", label: m.settings_database },
		{ id: "about", label: m.settings_about },
	];

	let advancedOpen = $state(false);
	let benchmarked = $state<number | null>(null);

	const number = (value: string, fallback: number) => {
		const n = Number(value);
		return Number.isFinite(n) ? n : fallback;
	};

	/** Editable parameters, only when the database already uses Argon2id. */
	const params: Argon2idParams | null = $derived.by(() => {
		const kdf = app.db?.kdf;
		if (kdf?.kind !== "argon2" || kdf.variant !== "argon2id") return null;
		return {
			memoryMib: Math.round(kdf.memoryBytes / (1024 * 1024)),
			iterations: kdf.iterations,
			parallelism: kdf.parallelism,
		};
	});

	const kdfLabel = $derived.by(() => {
		const kdf = app.db?.kdf;
		if (kdf?.kind === "aesKdf") return m.kdf_current_aes({ rounds: String(kdf.rounds) });
		if (kdf?.kind === "argon2")
			return m.kdf_current_argon2d({
				memory: String(Math.round(kdf.memoryBytes / (1024 * 1024))),
				iterations: String(kdf.iterations),
				parallelism: String(kdf.parallelism),
			});
		return m.kdf_current_other();
	});

	async function setKdf(patch: Partial<Argon2idParams>) {
		if (!params) return;
		await app.updateDatabase({ kdf: { ...params, ...patch } });
	}

	/** Measures this computer; also how AES-KDF / Argon2d switch to Argon2id. */
	async function benchmark() {
		const memoryMib = params?.memoryMib ?? 64;
		const parallelism = params?.parallelism ?? 2;
		const iterations = await app.benchmark(memoryMib, parallelism);
		if (iterations !== null) {
			benchmarked = iterations;
			await app.updateDatabase({ kdf: { memoryMib, iterations, parallelism } });
		}
	}

	// Change master password dialog.
	let newPassword = $state("");
	let confirmPassword = $state("");
	let passwordOutcome = $state<"notDurable" | "pending" | "rejected" | null>(null);
	let changingPassword = $state(false);
	let newStrength = $state<StrengthEstimate>(estimateStrength(""));
	let strengthRequest = 0;
	$effect(() => {
		const value = newPassword;
		const request = ++strengthRequest;
		void app.api.vault.estimateStrength(value).then((s) => {
			if (request === strengthRequest) newStrength = s;
		});
	});
	const passwordsMatch = $derived(newPassword.length > 0 && newPassword === confirmPassword);

	// The typed passwords live only while the dialog is open: closing it in
	// any way (Lock included) wipes them.
	$effect(() => {
		if (app.overlay !== "masterPassword") {
			newPassword = "";
			confirmPassword = "";
			passwordOutcome = null;
		}
	});

	function closeMasterPassword() {
		newPassword = "";
		confirmPassword = "";
		passwordOutcome = null;
		app.openSettings("database");
	}

	async function submitMasterPassword(event: SubmitEvent) {
		event.preventDefault();
		if (!passwordsMatch || changingPassword) return;
		const secret = newPassword;
		changingPassword = true;
		const outcome = await app.changeMasterPassword(secret);
		changingPassword = false;
		// Locked or closed meanwhile: nothing to show, nothing to reopen.
		if (app.overlay !== "masterPassword" || app.screen !== "vault") return;
		if (outcome === "changed") closeMasterPassword();
		else passwordOutcome = outcome;
	}

	async function discardPasswordChange() {
		await app.discardUnsaved();
		closeMasterPassword();
	}

	const input =
		"h-[34px] rounded-lg border border-input bg-background px-2 outline-none focus:border-ring focus:ring-3 focus:ring-ring/20";
	const heading = "text-xs font-semibold tracking-wider text-muted-foreground uppercase";
	const row =
		"flex items-center justify-between gap-4 border-b border-muted py-2.5 last:border-b-0";
</script>

<Dialog.Root {open} onOpenChange={(o) => !o && app.closeOverlay()}>
	<Dialog.Content
		class="flex h-[540px] gap-0 overflow-hidden p-0 sm:max-w-[760px]"
		showCloseButton={false}
	>
		<div
			class="flex w-[200px] shrink-0 flex-col gap-0.5 border-r border-border bg-surface px-2 py-4"
		>
			<Dialog.Title class="px-2.5 pt-1 pb-3 text-sm font-semibold"
				>{m.settings_title()}</Dialog.Title
			>
			<div role="tablist" aria-orientation="vertical" class="flex flex-col gap-0.5">
				{#each tabs as tab (tab.id)}
					<button
						type="button"
						role="tab"
						id="settings-tab-{tab.id}"
						aria-selected={app.settingsTab === tab.id}
						aria-controls="settings-panel"
						class="h-8 rounded-md px-2.5 text-left {app.settingsTab === tab.id
							? 'bg-selection font-medium text-selection-foreground'
							: 'text-sidebar-foreground hover:bg-muted'}"
						onclick={() => (app.settingsTab = tab.id)}>{tab.label()}</button
					>
				{/each}
			</div>
		</div>
		<div class="flex min-w-0 flex-1 flex-col">
			<div
				id="settings-panel"
				role="tabpanel"
				aria-labelledby="settings-tab-{app.settingsTab}"
				class="flex flex-1 flex-col gap-5 overflow-y-auto px-7 py-6"
			>
				{#if app.settingsTab === "general"}
					<h2 class="text-base font-semibold">{m.settings_general()}</h2>
					<section class="flex flex-col">
						<h3 class={heading}>{m.settings_clipboard()}</h3>
						<div class={row}>
							<label for="settings-clipboard">{m.settings_clipboard_clear()}</label>
							<div class="flex items-center gap-2">
								<input
									id="settings-clipboard"
									type="number"
									min="5"
									max="300"
									class="{input} w-20 text-right"
									value={app.settings.clipboard_clear_secs}
									onchange={(e) =>
										void app.updateSettings({
											clipboard_clear_secs: number(e.currentTarget.value, 20),
										})}
								/>
								<span class="text-muted-foreground">{m.settings_seconds()}</span>
							</div>
						</div>
					</section>
					<section class="flex flex-col">
						<h3 class={heading}>{m.settings_autolock()}</h3>
						<div class={row}>
							<label for="settings-idle">{m.settings_idle()}</label>
							<div class="flex items-center gap-2">
								<input
									id="settings-idle"
									type="number"
									min="1"
									max="240"
									class="{input} w-20 text-right"
									value={app.settings.idle_lock_mins}
									onchange={(e) =>
										void app.updateSettings({
											idle_lock_mins: number(e.currentTarget.value, 5),
										})}
								/>
								<span class="text-muted-foreground">{m.settings_minutes()}</span>
							</div>
						</div>
						<div class={row}>
							<label for="settings-screen-lock">{m.settings_lock_screen()}</label>
							<Switch
								id="settings-screen-lock"
								checked={app.settings.lock_on_screen_lock}
								onCheckedChange={(v) =>
									void app.updateSettings({ lock_on_screen_lock: v })}
							/>
						</div>
						<div class={row}>
							<label for="settings-minimize">{m.settings_lock_minimize()}</label>
							<Switch
								id="settings-minimize"
								checked={app.settings.lock_on_minimize}
								onCheckedChange={(v) =>
									void app.updateSettings({ lock_on_minimize: v })}
							/>
						</div>
					</section>
					<section class="flex flex-col">
						<h3 class={heading}>{m.generator_title()}</h3>
						<div class={row}>
							<label for="settings-length">{m.settings_generator_length()}</label>
							<input
								id="settings-length"
								type="number"
								min="8"
								max="128"
								class="{input} w-20 text-right"
								value={app.settings.generator.length}
								onchange={(e) =>
									void app.updateSettings({
										generator: {
											...app.settings.generator,
											length: number(e.currentTarget.value, 20),
										},
									})}
							/>
						</div>
					</section>
				{:else if app.settingsTab === "database"}
					<h2 class="text-base font-semibold">{m.settings_database()}</h2>
					{#if !app.db}
						<p class="text-muted-foreground">{m.settings_db_locked()}</p>
					{:else}
						<div class="flex flex-col gap-1.5">
							<label for="settings-db-name" class="text-[13px] font-medium"
								>{m.settings_db_name()}</label
							>
							<input
								id="settings-db-name"
								class="{input} h-9 px-3"
								value={app.db.name}
								onchange={(e) => {
									const name = e.currentTarget.value.trim();
									if (name) void app.updateDatabase({ name });
								}}
							/>
						</div>
						<div class="flex flex-col gap-1.5">
							<label for="settings-cipher" class="text-[13px] font-medium"
								>{m.cipher_label()}</label
							>
							<select
								id="settings-cipher"
								class="{input} h-9"
								value={app.db.cipher}
								onchange={(e) =>
									void app.updateDatabase({
										cipher: e.currentTarget.value as Cipher,
									})}
							>
								<option value="aes256">{m.cipher_aes()}</option>
								<option value="chaCha20">{m.cipher_chacha()}</option>
								{#if app.db.cipher === "twofish"}<option value="twofish"
										>{m.cipher_twofish()}</option
									>{/if}
							</select>
						</div>
						<div class="flex items-center justify-between gap-4">
							<label for="settings-recycle">{m.settings_db_recycle_bin()}</label>
							<Switch
								id="settings-recycle"
								checked={app.db.recycleBin}
								onCheckedChange={(v) => void app.updateDatabase({ recycleBin: v })}
							/>
						</div>
						<div class="rounded-[10px] border border-border">
							<button
								type="button"
								class="flex h-11 w-full items-center justify-between rounded-[10px] px-3.5 font-medium hover:bg-surface"
								aria-expanded={advancedOpen}
								aria-controls="settings-advanced"
								onclick={() => (advancedOpen = !advancedOpen)}
							>
								{m.settings_db_advanced()}
								<ChevronDown
									class="size-4 text-muted-foreground transition-transform {advancedOpen
										? 'rotate-180'
										: ''}"
									aria-hidden="true"
								/>
							</button>
							{#if advancedOpen}
								<div
									id="settings-advanced"
									class="flex flex-col gap-3 px-3.5 pt-1 pb-3.5"
								>
									{#if params}
										<p class="text-[13px] text-muted-foreground">
											{m.kdf_explanation()}
										</p>
										<div class="grid grid-cols-3 gap-3">
											<label class="flex flex-col gap-1 text-[13px]"
												>{m.kdf_memory()}<input
													type="number"
													min="8"
													class="{input} font-mono"
													value={params.memoryMib}
													onchange={(e) =>
														void setKdf({
															memoryMib: number(
																e.currentTarget.value,
																64,
															),
														})}
												/></label
											>
											<label class="flex flex-col gap-1 text-[13px]"
												>{m.kdf_iterations()}<input
													type="number"
													min="1"
													class="{input} font-mono"
													value={params.iterations}
													onchange={(e) =>
														void setKdf({
															iterations: number(
																e.currentTarget.value,
																3,
															),
														})}
												/></label
											>
											<label class="flex flex-col gap-1 text-[13px]"
												>{m.kdf_parallelism()}<input
													type="number"
													min="1"
													class="{input} font-mono"
													value={params.parallelism}
													onchange={(e) =>
														void setKdf({
															parallelism: number(
																e.currentTarget.value,
																2,
															),
														})}
												/></label
											>
										</div>
										<div class="flex items-center gap-3">
											<Button
												variant="outline"
												disabled={app.busy === "benchmark"}
												onclick={() => void benchmark()}
											>
												{app.busy === "benchmark"
													? m.kdf_benchmark_busy()
													: m.kdf_benchmark()}
											</Button>
											{#if benchmarked !== null}
												<span
													class="text-xs text-muted-foreground"
													role="status"
													>{m.kdf_benchmark_result({
														iterations: String(benchmarked),
													})}</span
												>
											{/if}
										</div>
									{:else}
										<p class="font-mono text-[13px]" data-testid="kdf-current">
											{kdfLabel}
										</p>
										<p class="text-[13px] text-muted-foreground">
											{m.kdf_switch_explanation()}
										</p>
										<Button
											variant="outline"
											class="self-start"
											disabled={app.busy === "benchmark"}
											onclick={() => void benchmark()}
										>
											{app.busy === "benchmark"
												? m.kdf_benchmark_busy()
												: m.kdf_switch()}
										</Button>
									{/if}
								</div>
							{/if}
						</div>
						<Button
							variant="outline"
							class="self-start"
							onclick={() => (app.overlay = "masterPassword")}
						>
							{m.settings_db_change_password()}
						</Button>
					{/if}
				{:else}
					<Wordmark
						size="lg"
						subtitle={app.appInfo
							? m.about_version({ version: app.appInfo.version })
							: m.about_version_unknown()}
					/>
					<p class="text-muted-foreground">{m.about_tagline()}</p>
					<p class="text-muted-foreground">{m.about_license()}</p>
					<div class="flex gap-4">
						<a
							class="font-medium text-primary hover:text-primary-hover"
							href="https://github.com/downlevel/keyrust"
							target="_blank"
							rel="noopener noreferrer">{m.about_source()}</a
						>
						<a
							class="font-medium text-primary hover:text-primary-hover"
							href="https://github.com/downlevel/keyrust/blob/main/SECURITY.md"
							target="_blank"
							rel="noopener noreferrer">{m.about_security()}</a
						>
					</div>
				{/if}
			</div>
			<div class="flex justify-end border-t border-muted px-5 py-3.5">
				<Button onclick={() => app.closeOverlay()}>{m.settings_done()}</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root
	open={app.overlay === "masterPassword"}
	onOpenChange={(o) => !o && closeMasterPassword()}
>
	<Dialog.Content class="sm:max-w-[440px]">
		<Dialog.Header>
			<Dialog.Title>{m.master_password_dialog_title()}</Dialog.Title>
			<Dialog.Description>{m.master_password_dialog_body()}</Dialog.Description>
		</Dialog.Header>
		<form class="flex flex-col gap-3" onsubmit={submitMasterPassword} novalidate>
			<div class="flex flex-col gap-1.5">
				<label for="new-master" class="text-[13px] font-medium"
					>{m.master_password_new()}</label
				>
				<SecretInput id="new-master" bind:value={newPassword} autocomplete="new-password" />
				<StrengthMeter estimate={newStrength} />
			</div>
			<div class="flex flex-col gap-1.5">
				<label for="confirm-master" class="text-[13px] font-medium"
					>{m.master_password_confirm()}</label
				>
				<SecretInput
					id="confirm-master"
					bind:value={confirmPassword}
					autocomplete="new-password"
					invalid={confirmPassword.length > 0 && !passwordsMatch}
					showCapsWarning={false}
				/>
				{#if confirmPassword.length > 0 && !passwordsMatch}
					<span class="text-xs text-destructive">{m.create_mismatch()}</span>
				{/if}
			</div>
			{#if passwordOutcome === "rejected"}
				<p class="text-[13px] text-destructive" role="alert">
					{m.master_password_failed()}
				</p>
			{:else if passwordOutcome === "notDurable"}
				<p class="text-[13px] text-warning-soft-foreground" role="alert">
					{m.master_password_not_durable()}
				</p>
			{:else if passwordOutcome === "pending"}
				<p class="text-[13px] text-warning-soft-foreground" role="alert">
					{m.master_password_pending()}
				</p>
			{/if}
			<Dialog.Footer>
				{#if passwordOutcome === "pending"}
					<Button
						type="button"
						variant="outline"
						onclick={() => void discardPasswordChange()}
						>{m.master_password_discard()}</Button
					>
					<Button type="button" onclick={closeMasterPassword}
						>{m.master_password_done()}</Button
					>
				{:else if passwordOutcome === "notDurable"}
					<Button type="button" onclick={closeMasterPassword}
						>{m.master_password_done()}</Button
					>
				{:else}
					<Button type="button" variant="outline" onclick={closeMasterPassword}
						>{m.master_password_cancel()}</Button
					>
					<Button type="submit" disabled={!passwordsMatch || changingPassword}
						>{m.master_password_save()}</Button
					>
				{/if}
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
