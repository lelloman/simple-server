"""Reject native transport dependencies in standalone engine consumer graphs."""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
NATIVE_FAMILIES = ("axum", "tokio", "reqwest", "hyper", "hyperlocal", "sqlx", "rustls", "multer")
FIXTURES = ("engine_consumer", "engine_drivers", "engine_lifecycle", "engine_tasks", "engine_web")


def is_native_dependency(name):
    return any(name == family or name.startswith(family + "-") for family in NATIVE_FAMILIES)


def native_dependencies(graph):
    """Parse cargo tree --prefix none; ignore paths, versions and duplicates."""
    return sorted({
        line.split()[0] for line in graph.splitlines()
        if line.strip() and is_native_dependency(line.split()[0])
    })


def main():
    for fixture in FIXTURES:
        graph = subprocess.check_output([
            "cargo", "tree", "--locked", "--manifest-path",
            str(ROOT / "tests/fixtures" / fixture / "Cargo.toml"),
            "--edges", "normal,build", "--prefix", "none",
        ], text=True)
        leaks = native_dependencies(graph)
        if leaks:
            print(f"Native engine dependencies leaked into {fixture}: {', '.join(leaks)}", file=sys.stderr)
            return 1
        print(f"{fixture}: no native transport dependencies")
    return 0


if __name__ == "__main__":
    sys.exit(main())
