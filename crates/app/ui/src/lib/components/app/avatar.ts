/** Stable colour pair (index into the --avatar-N tokens) for an entry title. */
export function avatarIndex(title: string): number {
	const code = title.trim().codePointAt(0) ?? 0;
	return code % 6;
}

export function initial(title: string): string {
	return [...title.trim()][0]?.toUpperCase() ?? "?";
}

export function domain(url: string): string {
	return url.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "").split(/[/?#]/)[0];
}
