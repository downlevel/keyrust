<script lang="ts">
	import Copy from "@lucide/svelte/icons/copy";
	import Eye from "@lucide/svelte/icons/eye";
	import EyeOff from "@lucide/svelte/icons/eye-off";
	import KeyRound from "@lucide/svelte/icons/key-round";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import type { GroupNode } from "$lib/api";
	import { Button } from "$lib/components/ui/button/index.js";
	import { formatShortcut } from "$lib/commands/commands";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";
	import { avatarIndex, initial } from "./avatar";
	import FieldRow from "./FieldRow.svelte";

	const app = useApp();

	const entry = $derived(app.selected);
	const detail = $derived(app.detail?.summary.id === app.selectedId ? app.detail : null);
	const tone = $derived(avatarIndex(entry?.title ?? ""));

	const groupPath = $derived.by(() => {
		if (!entry) return "";
		const names: string[] = [];
		const find = (nodes: GroupNode[], trail: string[]): boolean => {
			for (const node of nodes) {
				if (node.id === entry.groupId) {
					names.push(...trail, node.isRecycleBin ? m.recycle_bin() : node.name);
					return true;
				}
				if (find(node.children, [...trail, node.name])) return true;
			}
			return false;
		};
		if (app.tree) {
			names.push(app.tree.name);
			if (entry.groupId !== app.tree.id) find(app.tree.children, []);
		}
		return names.join(" › ");
	});

	const dateFormat = new Intl.DateTimeFormat("en", {
		day: "numeric",
		month: "short",
		year: "numeric",
	});
	const date = (iso: string) => {
		const d = new Date(iso);
		return Number.isNaN(d.getTime()) ? iso : dateFormat.format(d);
	};

	/** Only http(s) links open in the system browser. */
	const safeUrl = $derived(entry && /^https?:\/\//i.test(entry.url) ? entry.url : null);

	const iconButton =
		"flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground";
</script>

{#if !entry}
	<div class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
		<KeyRound class="size-6" aria-hidden="true" strokeWidth={1.8} />
		<span>{m.no_selection()}</span>
	</div>
{:else}
	<article class="flex max-w-[720px] flex-col gap-6 px-10 py-8" aria-labelledby="entry-title">
		<header class="flex items-center gap-3.5">
			<span
				class="flex size-11 shrink-0 items-center justify-center rounded-[10px] text-lg font-semibold"
				style="background: var(--avatar-{tone}-bg); color: var(--avatar-{tone}-fg)"
				aria-hidden="true">{initial(entry.title)}</span
			>
			<div class="flex min-w-0 flex-1 flex-col">
				<h1 id="entry-title" class="truncate text-xl font-semibold tracking-tight">
					{entry.title}
				</h1>
				<span class="truncate text-xs text-muted-foreground">{groupPath}</span>
			</div>
			{#if app.inRecycleBin}
				<Button variant="outline" onclick={() => void app.restoreSelected()}
					>{m.restore()}</Button
				>
				<Button
					variant="outline"
					size="icon"
					aria-label={m.delete()}
					title={`${m.delete()} (${formatShortcut("Delete")})`}
					onclick={() => app.askDeleteEntry()}
				>
					<Trash2 class="size-4" aria-hidden="true" />
				</Button>
			{:else}
				<Button
					variant="outline"
					size="icon"
					aria-label={m.delete()}
					title={`${m.delete()} (${formatShortcut("Delete")})`}
					onclick={() => app.askDeleteEntry()}
				>
					<Trash2 class="size-4" aria-hidden="true" />
				</Button>
				<Button
					title={`${m.edit()} (${formatShortcut("CmdOrCtrl+E")})`}
					onclick={() => void app.editEntry()}
				>
					<Pencil class="size-3.5" aria-hidden="true" />
					{m.edit()}
				</Button>
			{/if}
		</header>

		<section class="flex flex-col overflow-hidden rounded-[10px] border border-border">
			<FieldRow label={m.field_username()}>
				<span class="block truncate">{entry.username || m.no_value()}</span>
				{#snippet actions()}
					<button
						type="button"
						class={iconButton}
						aria-label={m.copy_username()}
						title={`${m.copy_username()} (${formatShortcut("CmdOrCtrl+B")})`}
						onclick={() => void app.copy("username")}
					>
						<Copy class="size-4" aria-hidden="true" />
					</button>
				{/snippet}
			</FieldRow>
			<FieldRow label={m.field_password()} alwaysShowActions>
				{#if detail && !detail.hasPassword}
					<span class="text-muted-foreground">{m.no_value()}</span>
				{:else}
					<span
						class="block truncate font-mono {app.revealed === null
							? 'tracking-[0.08em]'
							: ''}"
						data-testid="password-value">{app.revealed ?? "••••••••••••"}</span
					>
				{/if}
				{#snippet actions()}
					<button
						type="button"
						class={iconButton}
						aria-label={app.revealed === null
							? m.reveal_password()
							: m.conceal_password()}
						aria-pressed={app.revealed !== null}
						title={app.revealed === null ? m.reveal_password() : m.conceal_password()}
						onclick={() => void app.toggleReveal()}
					>
						{#if app.revealed === null}<Eye
								class="size-4"
								aria-hidden="true"
							/>{:else}<EyeOff class="size-4" aria-hidden="true" />{/if}
					</button>
					<button
						type="button"
						class={iconButton}
						aria-label={m.copy_password()}
						title={`${m.copy_password()} (${formatShortcut("CmdOrCtrl+C")})`}
						onclick={() => void app.copy("password")}
					>
						<Copy class="size-4" aria-hidden="true" />
					</button>
				{/snippet}
			</FieldRow>
			<FieldRow label={m.field_url()}>
				{#if safeUrl}
					<!-- External link to the entry's site, opened in the system browser. -->
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						href={safeUrl}
						target="_blank"
						rel="noopener noreferrer"
						class="block truncate text-primary hover:text-primary-hover"
						onclick={(e) => app.isDemo && e.preventDefault()}>{entry.url}</a
					>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{:else}
					<span class="block truncate">{entry.url || m.no_value()}</span>
				{/if}
				{#snippet actions()}
					<button
						type="button"
						class={iconButton}
						aria-label={m.copy_url()}
						title={`${m.copy_url()} (${formatShortcut("CmdOrCtrl+U")})`}
						onclick={() => void app.copy("url")}
					>
						<Copy class="size-4" aria-hidden="true" />
					</button>
				{/snippet}
			</FieldRow>
			<FieldRow label={m.field_notes()} multiline>
				{#if detail?.notes}
					<p class="whitespace-pre-line">{detail.notes}</p>
				{:else}
					<p class="text-muted-foreground">{m.no_notes()}</p>
				{/if}
			</FieldRow>
		</section>

		{#if detail}
			<footer class="flex gap-6 px-1 text-xs text-muted-foreground">
				<span>{m.meta_created({ date: date(detail.created) })}</span>
				<span>{m.meta_modified({ date: date(detail.modified) })}</span>
			</footer>
		{/if}
	</article>
{/if}
