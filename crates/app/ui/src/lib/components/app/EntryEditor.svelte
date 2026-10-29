<script lang="ts">
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import type { StrengthEstimate } from "$lib/api";
	import { estimateStrength } from "$lib/api/strength";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Popover from "$lib/components/ui/popover/index.js";
	import { formatShortcut } from "$lib/commands/commands";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";
	import PasswordGenerator from "./PasswordGenerator.svelte";
	import SecretInput from "./SecretInput.svelte";
	import StrengthMeter from "./StrengthMeter.svelte";

	const app = useApp();
	const editor = $derived(app.editor);

	let attempted = $state(false);

	let strength = $state<StrengthEstimate>(estimateStrength(""));
	let sequence = 0;
	$effect(() => {
		const value = app.editor?.draft.password ?? "";
		const current = ++sequence;
		void app.api.vault.estimateStrength(value).then((s) => {
			if (current === sequence) strength = s;
		});
	});

	const titleMissing = $derived(!!editor && !editor.draft.title.trim());

	const groupOptions = $derived(
		app.groups
			.filter((g) => !g.isRecycleBin && !g.inRecycleBin)
			.map((g) => ({ id: g.id, label: `${" ".repeat(g.depth + 1)}${g.name}` })),
	);

	const generatorOpen = $derived(app.overlay === "generator");

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		attempted = true;
		if (titleMissing) return;
		await app.saveEditor();
	}

	const field =
		"h-[38px] rounded-lg border border-input bg-background px-3 outline-none focus:border-ring focus:ring-2 focus:ring-ring";
</script>

{#if editor}
	<form class="flex max-w-[720px] flex-col gap-[18px] px-10 py-8" onsubmit={submit} novalidate>
		<fieldset class="contents" disabled={app.savingEntry}>
			<h1 class="text-xl font-semibold tracking-tight">
				{editor.isNew ? m.editor_new() : m.editor_edit()}
			</h1>

			<div class="flex flex-col gap-1.5">
				<label for="editor-title" class="text-[13px] font-medium">{m.field_title()}</label>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					id="editor-title"
					class="{field} {attempted && titleMissing ? 'border-destructive' : ''}"
					value={editor.draft.title}
					placeholder={m.editor_title_placeholder()}
					aria-invalid={(attempted && titleMissing) || undefined}
					aria-describedby={attempted && titleMissing ? "editor-title-error" : undefined}
					autofocus
					oninput={(e) => app.updateDraft({ title: e.currentTarget.value })}
				/>
				{#if attempted && titleMissing}
					<span id="editor-title-error" class="text-[13px] text-destructive"
						>{m.editor_title_required()}</span
					>
				{/if}
			</div>

			<div class="flex flex-col gap-1.5">
				<label for="editor-username" class="text-[13px] font-medium"
					>{m.field_username()}</label
				>
				<input
					id="editor-username"
					class={field}
					value={editor.draft.username}
					autocomplete="off"
					spellcheck={false}
					oninput={(e) => app.updateDraft({ username: e.currentTarget.value })}
				/>
			</div>

			<div class="flex flex-col gap-1.5">
				<label for="editor-password" class="text-[13px] font-medium"
					>{m.field_password()}</label
				>
				<div class="flex items-start gap-2">
					<div class="flex-1">
						<SecretInput
							id="editor-password"
							value={editor?.draft.password ?? ""}
							showCapsWarning={false}
							describedby="editor-password-strength"
							oninput={(value) => app.updateDraft({ password: value })}
						/>
					</div>
					<Popover.Root
						open={generatorOpen}
						onOpenChange={(open) => (open ? app.openGenerator() : app.closeOverlay())}
					>
						<Popover.Trigger
							class="flex h-10 items-center gap-1.5 rounded-lg border border-input bg-background px-3 font-medium hover:bg-muted"
							title={`${m.generator_title()} (${formatShortcut("CmdOrCtrl+G")})`}
						>
							<RefreshCw class="size-4" aria-hidden="true" />
							{m.editor_generate()}
						</Popover.Trigger>
						<Popover.Content
							class="w-[380px] rounded-xl p-4"
							align="end"
							role="dialog"
							aria-label={m.generator_title()}
						>
							<PasswordGenerator />
						</Popover.Content>
					</Popover.Root>
				</div>
				<StrengthMeter estimate={strength} id="editor-password-strength" />
			</div>

			<div class="flex flex-col gap-1.5">
				<label for="editor-url" class="text-[13px] font-medium">{m.field_url()}</label>
				<input
					id="editor-url"
					class={field}
					value={editor.draft.url}
					placeholder="https://"
					autocomplete="off"
					spellcheck={false}
					oninput={(e) => app.updateDraft({ url: e.currentTarget.value })}
				/>
			</div>

			<div class="flex flex-col gap-1.5">
				<label for="editor-group" class="text-[13px] font-medium">{m.field_group()}</label>
				<select
					id="editor-group"
					class="{field} px-2.5"
					value={editor.draft.groupId}
					onchange={(e) => app.updateDraft({ groupId: e.currentTarget.value })}
				>
					{#if app.tree}<option value={app.tree.id}>{app.tree.name}</option>{/if}
					{#each groupOptions as option (option.id)}
						<option value={option.id}>{option.label}</option>
					{/each}
				</select>
			</div>

			<div class="flex flex-col gap-1.5">
				<label for="editor-notes" class="text-[13px] font-medium">{m.field_notes()}</label>
				<textarea
					id="editor-notes"
					class="min-h-24 resize-y rounded-lg border border-input bg-background px-3 py-2.5 outline-none focus:border-ring focus:ring-2 focus:ring-ring"
					rows="4"
					value={editor.draft.notes}
					oninput={(e) => app.updateDraft({ notes: e.currentTarget.value })}></textarea>
			</div>

			<div class="flex justify-end gap-2 pt-1">
				<Button
					type="button"
					variant="outline"
					title={`${m.editor_cancel()} (Esc)`}
					onclick={() => app.cancelEditor()}>{m.editor_cancel()}</Button
				>
				<Button
					type="submit"
					title={`${m.editor_save()} (${formatShortcut("CmdOrCtrl+S")})`}
					>{m.editor_save()}</Button
				>
			</div>
		</fieldset>
	</form>
{/if}
