<script lang="ts">
	import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
	import { Button } from "$lib/components/ui/button/index.js";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();
	const c = $derived(app.confirm);

	const text = $derived.by(() => {
		if (!c) return null;
		if (c.kind === "deleteEntry") {
			return c.permanent
				? {
						title: m.confirm_destroy_entry_title({ title: c.title }),
						body: m.confirm_destroy_entry_body(),
						action: m.confirm_destroy_entry_action(),
					}
				: {
						title: m.confirm_delete_entry_title({ title: c.title }),
						body: m.confirm_delete_entry_body(),
						action: m.confirm_delete_entry_action(),
					};
		}
		if (c.kind === "deleteGroup") {
			return {
				title: m.confirm_delete_group_title({ name: c.name }),
				body: m.confirm_delete_group_body(),
				action: m.confirm_delete_group_action(),
			};
		}
		return { title: m.leave_title(), body: m.leave_body(), action: m.leave_save() };
	});

	function dismiss() {
		if (c?.kind === "leave") void app.resolveLeave("keep");
		else app.confirm = null;
	}
</script>

<AlertDialog.Root open={!!c} onOpenChange={(open) => !open && dismiss()}>
	<AlertDialog.Content class="sm:max-w-[420px]">
		{#if c && text}
			<AlertDialog.Header>
				<AlertDialog.Title>{text.title}</AlertDialog.Title>
				<AlertDialog.Description>{text.body}</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				{#if c.kind === "leave"}
					<Button variant="outline" onclick={() => void app.resolveLeave("keep")}
						>{m.leave_keep()}</Button
					>
					<Button variant="outline" onclick={() => void app.resolveLeave("discard")}
						>{m.leave_discard()}</Button
					>
					<Button onclick={() => void app.resolveLeave("save")}>{m.leave_save()}</Button>
				{:else}
					<Button variant="outline" onclick={dismiss}>{m.confirm_cancel()}</Button>
					<Button
						class="bg-destructive text-destructive-foreground hover:bg-destructive-hover"
						onclick={() => void app.confirmDelete()}>{text.action}</Button
					>
				{/if}
			</AlertDialog.Footer>
		{/if}
	</AlertDialog.Content>
</AlertDialog.Root>
