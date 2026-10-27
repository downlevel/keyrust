import { getContext, setContext } from "svelte";
import type { AppState } from "./app.svelte";

const KEY = Symbol("keyrust-app");

export function provideApp(state: AppState): AppState {
	return setContext(KEY, state);
}

export function useApp(): AppState {
	return getContext<AppState>(KEY);
}
