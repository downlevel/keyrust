<script lang="ts">
	import FileText from "@lucide/svelte/icons/file-text";
	import LoaderCircle from "@lucide/svelte/icons/loader-circle";
	import { Button } from "$lib/components/ui/button/index.js";
	import DemoBadge from "$lib/components/app/DemoBadge.svelte";
	import SecretInput from "$lib/components/app/SecretInput.svelte";
	import Wordmark from "$lib/components/app/Wordmark.svelte";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();

	let password = $state("");
	let input = $state<HTMLInputElement | null>(null);

	const recent = $derived(app.recents[app.recentIndex]);
	const unlocking = $derived(app.busy === "unlock");
	const errorText = $derived(
		app.unlockError === "wrongPassword"
			? m.unlock_error_wrong()
			: app.unlockError === "notFound"
				? m.unlock_error_not_found()
				: "",
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (unlocking) return;
		const value = password;
		// The field never keeps the master password after an attempt.
		password = "";
		await app.unlock(value);
		input?.focus();
	}

	function pick(index: number) {
		password = "";
		app.selectRecent(index);
		input?.focus();
	}
</script>

<main class="flex flex-1 items-center justify-center overflow-y-auto p-8">
	<div class="flex w-[400px] flex-col gap-6">
		{#if app.isDemo}<DemoBadge detailed />{/if}
		{#if app.screen === "welcome"}
			<section
				class="flex flex-col gap-4 rounded-xl border border-border bg-card p-8 shadow-xs"
			>
				<Wordmark />
				<h1 class="text-base font-semibold">{m.welcome_title()}</h1>
				<p class="text-muted-foreground">{m.welcome_body()}</p>
				<Button size="lg" class="h-10" onclick={() => app.startCreate()}
					>{m.welcome_create()}</Button
				>
				<Button
					size="lg"
					variant="outline"
					class="h-10"
					onclick={() => void app.openOtherFile()}>{m.welcome_open()}</Button
				>
			</section>
		{:else if recent}
			<section
				class="flex flex-col gap-5 rounded-xl border border-border bg-card p-8 shadow-xs"
			>
				<Wordmark subtitle={m.unlock_subtitle()} />
				<div
					class="flex items-center gap-2.5 rounded-lg border border-border bg-surface px-3 py-2.5"
				>
					<FileText class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
					<div class="flex min-w-0 flex-col">
						<span class="font-medium" data-testid="unlock-db-name">{recent.name}</span>
						<span class="truncate text-xs text-muted-foreground">{recent.path}</span>
					</div>
				</div>
				<form class="flex flex-col gap-2" onsubmit={submit}>
					<label for="master-password" class="text-[13px] font-medium"
						>{m.unlock_master_password_label()}</label
					>
					<SecretInput
						id="master-password"
						bind:value={password}
						bind:ref={input}
						placeholder={m.unlock_placeholder()}
						disabled={unlocking}
						invalid={!!errorText}
						describedby="master-password-error"
						autofocus
					/>
					<div
						id="master-password-error"
						aria-live="polite"
						class="min-h-5 text-[13px] text-destructive"
					>
						{errorText}
					</div>
					<Button type="submit" size="lg" class="h-10" disabled={unlocking}>
						{#if unlocking}
							<LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
							{m.unlock_busy()}
						{:else}
							{m.unlock_button()}
						{/if}
					</Button>
				</form>
			</section>
			<section class="flex flex-col gap-2" aria-labelledby="recent-heading">
				<h2
					id="recent-heading"
					class="px-1 text-xs font-semibold tracking-wider text-muted-foreground uppercase"
				>
					{m.recent_heading()}
				</h2>
				<ul class="flex flex-col gap-0.5">
					{#each app.recents as file, index (file.path)}
						<li>
							<button
								type="button"
								class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left {index ===
								app.recentIndex
									? 'bg-card shadow-xs ring-1 ring-border'
									: 'hover:bg-muted'}"
								aria-current={index === app.recentIndex || undefined}
								onclick={() => pick(index)}
							>
								<FileText
									class="size-4 shrink-0 {index === app.recentIndex
										? 'text-primary'
										: 'text-muted-foreground'}"
									aria-hidden="true"
								/>
								<span class="flex min-w-0 flex-1 flex-col">
									<span class="font-medium">{file.name}</span>
									<span class="truncate text-xs text-muted-foreground"
										>{file.path}</span
									>
								</span>
								{#if file.missing}
									<span class="text-xs text-muted-foreground"
										>{m.recent_missing()}</span
									>
								{/if}
							</button>
						</li>
					{/each}
				</ul>
				<div class="flex gap-4 px-1 pt-2">
					<button
						type="button"
						class="text-[13px] font-medium text-primary hover:text-primary-hover"
						onclick={() => void app.openOtherFile()}>{m.open_other_file()}</button
					>
					<button
						type="button"
						class="text-[13px] font-medium text-primary hover:text-primary-hover"
						onclick={() => app.startCreate()}>{m.create_new_database()}</button
					>
				</div>
			</section>
		{/if}
	</div>
</main>
