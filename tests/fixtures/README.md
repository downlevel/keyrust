# Test fixtures

Test-only KeePass databases. **They contain no real credentials.** The password of
every fixture is `keyrust-test-password` (public by design).

| File | Created by | Format | Cipher / KDF | Source |
|---|---|---|---|---|
| `keepassxc-rich.kdbx` | KeePassXC 2.7.6 (`keepassxc-cli import`) | KDBX 4.1 | AES-256 / AES-KDF (rounds benchmarked for ~100 ms by `-t 100`, so they vary by machine) | `src/rich.xml` |
| `keepassxc-kdbx31.kdbx` | KeePassXC 2.7.6 (`keepassxc-cli db-create` + `add`) | KDBX 3.1 | AES-256 / AES-KDF (`-t 100`) | one entry `Legacy` (user `alice`, password `legacy-entry-password`) |

`src/rich.xml` is a hand-written KeePass 2 XML file covering every capability the
compatibility spike checks: nested groups, standard and custom (protected) fields,
Unicode notes, tags, text and binary attachments, entry history, AutoType, entry and
database CustomData, a custom icon, the recycle bin and a `DeletedObjects` tombstone.

Copies of these files seed the fuzzing corpus in `fuzz/corpus/`.

Regenerate with:

```sh
scripts/gen-fixtures.sh
```

The encrypted bytes change on every run (random seeds), the content does not; the
tests in `crates/core/tests/compat_keepassxc.rs` must stay green.

Still missing (needs the KeePassXC GUI, which lets you pick the KDF and cipher):
databases created by KeePassXC with Argon2id + ChaCha20 and with Argon2d.
