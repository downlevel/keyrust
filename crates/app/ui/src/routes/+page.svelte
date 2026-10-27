<script lang="ts">
	import { createApis } from "$lib/api";
	import App from "$lib/App.svelte";

	const apis = createApis();
</script>

<App {apis} />
