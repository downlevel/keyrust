<script lang="ts">
	import Eye from "@lucide/svelte/icons/eye";
	import EyeOff from "@lucide/svelte/icons/eye-off";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import { m } from "$lib/paraglide/messages.js";

	let {
		id,
		value = $bindable(""),
		placeholder = "",
		disabled = false,
		invalid = false,
		describedby,
		autofocus = false,
		autocomplete = "off",
		showCapsWarning = true,
		ref = $bindable(null),
		oninput,
	}: {
		id: string;
		value?: string;
		placeholder?: string;
		disabled?: boolean;
		invalid?: boolean;
		describedby?: string;
		autofocus?: boolean;
		autocomplete?: "off" | "new-password";
		showCapsWarning?: boolean;
		ref?: HTMLInputElement | null;
		oninput?: (value: string) => void;
	} = $props();

	let visible = $state(false);
	let capsLock = $state(false);

	function updateCaps(event: KeyboardEvent) {
		if (typeof event.getModifierState === "function")
			capsLock = event.getModifierState("CapsLock");
	}

	$effect(() => {
		if (autofocus && ref && !disabled) ref.focus();
	});
</script>

<div class="flex flex-col gap-1.5">
	<div
		class="flex items-center rounded-lg border bg-background pr-1 focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/20 {invalid
			? 'border-destructive'
			: 'border-input'} {disabled ? 'opacity-60' : ''}"
	>
		<input
			{id}
			bind:this={ref}
			bind:value
			type={visible ? "text" : "password"}
			class="h-10 min-w-0 flex-1 rounded-lg border-0 bg-transparent px-3 outline-none placeholder:text-muted-foreground {visible
				? 'font-mono'
				: ''}"
			{placeholder}
			{disabled}
			{autocomplete}
			spellcheck={false}
			autocapitalize="off"
			aria-invalid={invalid || undefined}
			aria-describedby={describedby}
			onkeydown={updateCaps}
			onkeyup={updateCaps}
			oninput={(e) => oninput?.(e.currentTarget.value)}
		/>
		<button
			type="button"
			class="flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
			aria-label={visible ? m.hide_password() : m.show_password()}
			aria-pressed={visible}
			{disabled}
			onclick={() => (visible = !visible)}
		>
			{#if visible}<EyeOff class="size-4" aria-hidden="true" />{:else}<Eye
					class="size-4"
					aria-hidden="true"
				/>{/if}
		</button>
	</div>
	{#if showCapsWarning && capsLock}
		<span
			class="inline-flex items-center gap-1.5 text-[13px] text-warning-soft-foreground"
			role="status"
		>
			<TriangleAlert class="size-3.5" aria-hidden="true" />
			{m.caps_lock_on()}
		</span>
	{/if}
</div>
