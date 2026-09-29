#!/usr/bin/env python3
"""Publish to the private Fucina Cargo registry without storing tokens in Cargo config."""

import argparse
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    env = os.environ.copy()
    token = env.get("FUCINA_CARGO_TOKEN")
    if not token:
        path = Path(env.get(
            "FUCINA_CARGO_TOKEN_FILE",
            "~/.config/fucina/simple-server-cargo-publish.token",
        )).expanduser()
        if path.stat().st_mode & 0o077:
            parser.error(f"credential file must be owner-only (chmod 600): {path}")
        token = path.read_text().strip()
    if not token.strip():
        parser.error("empty Fucina Cargo token")
    env["CARGO_REGISTRIES_FUCINA_TOKEN"] = (
        token if token.startswith("Bearer ") else "Bearer " + token
    )
    command = ["cargo", "publish", "--locked", "--registry", "fucina"]
    if args.dry_run:
        command.append("--dry-run")
    result = subprocess.run(command, cwd=Path(__file__).resolve().parents[1], env=env)
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
