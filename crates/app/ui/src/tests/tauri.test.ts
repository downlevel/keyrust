import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri: () => true }));

const { TauriVault } = await import("$lib/api/tauri");
const { VaultError } = await import("$lib/api/vault");

beforeEach(() => invoke.mockReset());

describe("TauriVault", () => {
	it("calls the Rust commands with their argument names", async () => {
		invoke.mockResolvedValue(undefined);
		const vault = new TauriVault();
		await vault.unlock("/a/b.kdbx", "pw");
		await vault.copyField("e1", "password");
		await vault.saveEntry({
			id: null,
			groupId: "g",
			title: "T",
			username: "",
			password: "",
			url: "",
			notes: "",
		});
		await vault.benchmarkKdf(64, 2);
		await vault.lock();
		await vault.create({
			path: "/a/c.kdbx",
			name: "C",
			password: "pw",
			cipher: "chaCha20",
			kdf: { memoryMib: 64, iterations: 3, parallelism: 2 },
		});
		expect(invoke.mock.calls).toEqual([
			["unlock", { path: "/a/b.kdbx", password: "pw" }],
			["copy_field", { id: "e1", field: "password" }],
			[
				"save_entry",
				{
					draft: {
						id: null,
						groupId: "g",
						title: "T",
						username: "",
						password: "",
						url: "",
						notes: "",
					},
				},
			],
			["benchmark_kdf", { memoryMib: 64, parallelism: 2 }],
			["lock_vault", undefined],
			[
				"create",
				{
					request: {
						path: "/a/c.kdbx",
						name: "C",
						password: "pw",
						cipher: "chaCha20",
						kdf: { memoryMib: 64, iterations: 3, parallelism: 2 },
					},
				},
			],
		]);
	});

	it("turns { kind } rejections into VaultError, anything else into io", async () => {
		const vault = new TauriVault();
		invoke.mockRejectedValueOnce({ kind: "externalChange" });
		await expect(vault.save()).rejects.toMatchObject({ kind: "externalChange" });
		invoke.mockRejectedValueOnce("something unexpected");
		const error = await vault.save().catch((e: unknown) => e);
		expect(error).toBeInstanceOf(VaultError);
		expect(error).toMatchObject({ kind: "io" });
	});

	it("lists carry no password fields", async () => {
		invoke.mockResolvedValueOnce([
			{ id: "e1", groupId: "g", title: "Bank", username: "alice", url: "https://bank" },
		]);
		const list = await new TauriVault().listEntries();
		expect(JSON.stringify(list)).not.toMatch(/password/i);
	});
});
