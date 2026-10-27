<script lang="ts">
	import Check from "@lucide/svelte/icons/check";
	import X from "@lucide/svelte/icons/x";
	import { useApp } from "$lib/state/context";
	import { m } from "$lib/paraglide/messages.js";

	const app = useApp();

	const fieldNames = {
		username: m.field_username,
		password: m.field_password,
		url: m.field_url,
		notes: m.field_notes,
	};
</script>

<!-- The live region always exists so that screen readers announce updates. -->
<div
	class="pointer-events-none fixed inset-x-0 bottom-6 z-50 flex justify-center"
	aria-live="polite"
>
	{#if app.toast}
		<div
			role="status"
			class="pointer-events-auto flex items-center gap-2.5 rounded-[10px] bg-toast py-2 pr-2 pl-3.5 text-[13px] text-toast-foreground shadow-lg"
		>
			<Check class="size-4 text-toast-icon" aria-hidden="true" strokeWidth={2.4} />
			<span
				>{m.toast_copied({
					field: fieldNames[app.toast.field](),
					secs: String(app.toast.secs),
				})}</span
			>
			<button
				type="button"
				class="flex size-6 items-center justify-center rounded-md text-toast-foreground/70 hover:bg-white/10 hover:text-toast-foreground"
				aria-label={m.toast_dismiss()}
				onclick={() => app.dismissToast()}
			>
				<X class="size-3.5" aria-hidden="true" />
			</button>
		</div>
	{/if}
</div>
