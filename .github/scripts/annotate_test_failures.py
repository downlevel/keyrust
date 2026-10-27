"""Turns failed Rust tests in a `cargo test` log into GitHub error annotations.

Annotations are visible on the run and the pull request without downloading
logs, which makes platform-specific failures quick to read.
"""

import re
import sys

MAX_ANNOTATIONS = 10
MAX_CHARS = 3000


def escape(text: str) -> str:
    return text.replace("%", "%25").replace("\r", "").replace("\n", "%0A")


def main(path: str) -> None:
    log = open(path, encoding="utf-8", errors="replace").read()
    blocks = re.findall(r"^---- (\S+) stdout ----\n(.*?)(?=^---- |^failures:|\Z)", log, re.S | re.M)
    for name, body in blocks[:MAX_ANNOTATIONS]:
        print(f"::error title=Test failed: {name}::{escape(body.strip()[:MAX_CHARS])}")
    if not blocks:
        tail = "\n".join(log.splitlines()[-40:])
        print(f"::error title=cargo test failed::{escape(tail[-MAX_CHARS:])}")


if __name__ == "__main__":
    main(sys.argv[1])
