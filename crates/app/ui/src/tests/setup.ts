import "@testing-library/jest-dom/vitest";

// jsdom lacks a few browser APIs that bits-ui and the layout rely on.
class ResizeObserverStub {
	observe() {}
	unobserve() {}
	disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver;
if (typeof Element !== "undefined") {
	Element.prototype.scrollIntoView ??= function () {};
	Element.prototype.hasPointerCapture ??= () => false;
	Element.prototype.releasePointerCapture ??= () => {};
}
if (typeof window !== "undefined")
	globalThis.matchMedia ??= ((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addListener() {},
		removeListener() {},
		addEventListener() {},
		removeEventListener() {},
		dispatchEvent: () => false,
	})) as typeof globalThis.matchMedia;
