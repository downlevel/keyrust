import registry from "./commands.json";
import { m } from "$lib/paraglide/messages.js";

export type CommandId = (typeof registry.commands)[number]["id"];

export interface Command {
	id: CommandId;
	menu: string | null;
	labelKey: string;
	/** Tauri accelerator syntax, e.g. `CmdOrCtrl+Shift+N`. */
	shortcut: string | null;
	scope: "global" | "outsideText";
	nativeAccelerator: boolean;
	trigger?: "keydown" | "copy";
	demoOnly?: boolean;
}

export const COMMANDS = registry.commands as Command[];
export const MENUS = registry.menus;

const BY_ID = new Map(COMMANDS.map((c) => [c.id, c]));

export function command(id: CommandId): Command {
	const c = BY_ID.get(id);
	if (!c) throw new Error(`unknown command ${id}`);
	return c;
}

export function isCommandId(id: string): id is CommandId {
	return BY_ID.has(id as CommandId);
}

type Messages = Record<string, () => string>;

export function commandLabel(c: Pick<Command, "labelKey">): string {
	return (m as unknown as Messages)[c.labelKey]();
}

export function menuLabel(labelKey: string): string {
	return (m as unknown as Messages)[labelKey]();
}

// ---------------------------------------------------------------------------
// Shortcuts
// ---------------------------------------------------------------------------

/** Accelerator key names (W3C code style, as Tauri expects) → `KeyboardEvent.key`. */
const KEY_NAMES: Record<string, string> = {
	Slash: "/",
	Comma: ",",
	Delete: "Delete",
	Enter: "Enter",
	Escape: "Escape",
};

interface Parsed {
	primary: boolean;
	shift: boolean;
	alt: boolean;
	key: string;
}

export function parseShortcut(shortcut: string): Parsed {
	const parts = shortcut.split("+");
	const name = parts.pop() ?? "";
	return {
		primary: parts.includes("CmdOrCtrl"),
		shift: parts.includes("Shift"),
		alt: parts.includes("Alt"),
		key: KEY_NAMES[name] ?? (name.length === 1 ? name.toLowerCase() : name),
	};
}

export function isMac(platform = globalThis.navigator?.platform ?? ""): boolean {
	return /mac|iphone|ipad/i.test(platform);
}

/** True when `event` is `shortcut`. CmdOrCtrl is ⌘ on macOS and Ctrl elsewhere. */
export function matchesShortcut(event: KeyboardEvent, shortcut: string, mac = isMac()): boolean {
	const s = parseShortcut(shortcut);
	const primary = mac ? event.metaKey : event.ctrlKey;
	const other = mac ? event.ctrlKey : event.metaKey;
	if (primary !== s.primary || other || event.altKey !== s.alt) return false;
	const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
	// Shift changes the produced character for symbols (Shift+7 is "/" on
	// some layouts), so it only has to match for letters and named keys.
	if (/^[a-z0-9]$|^[A-Z]/.test(s.key) && event.shiftKey !== s.shift) return false;
	return key === s.key;
}

/** Human-readable shortcut: `Ctrl+Shift+N` or `⇧⌘N`. */
export function formatShortcut(shortcut: string, mac = isMac()): string {
	const s = parseShortcut(shortcut);
	const key = s.key.length === 1 ? s.key.toUpperCase() : s.key === "Delete" ? "Del" : s.key;
	if (mac) return `${s.alt ? "⌥" : ""}${s.shift ? "⇧" : ""}${s.primary ? "⌘" : ""}${key}`;
	return [s.primary && "Ctrl", s.alt && "Alt", s.shift && "Shift", key].filter(Boolean).join("+");
}

/** Focus is somewhere text is typed, so plain shortcuts belong to the field. */
export function isEditableTarget(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false;
	if (target.isContentEditable) return true;
	if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
	if (target instanceof HTMLInputElement) {
		return !["checkbox", "radio", "button", "submit", "range", "reset"].includes(target.type);
	}
	return false;
}

/** The keydown command for `event`, honouring each command's scope. */
export function commandForKeydown(event: KeyboardEvent, mac = isMac()): Command | null {
	const editable = isEditableTarget(event.target);
	for (const c of COMMANDS) {
		if (!c.shortcut || c.trigger === "copy") continue;
		if (c.scope === "outsideText" && editable) continue;
		if (matchesShortcut(event, c.shortcut, mac)) return c;
	}
	return null;
}
