"""Guard the standalone scheduler's normal/build graph, excluding dev features."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
graph = subprocess.check_output([
    "cargo", "tree", "--locked", "--manifest-path",
    str(root / "tests/fixtures/engine_scheduler/Cargo.toml"),
    "--edges", "normal,build", "--prefix", "none", "--format", "{p}|{f}",
], text=True)
found = False
packages = set()
for line in graph.splitlines():
    package, features = line.split("|", 1)
    name = package.split()[0]
    packages.add(tuple(package.split()[:2]))
    assert name not in {"axum", "reqwest", "hyper", "sqlx", "rustls", "mio", "socket2"}, line
    if name == "tokio":
        found = True
        enabled = set(features.removesuffix(" (*)").split(","))
        assert enabled <= {"macros", "sync", "tokio-macros"}, line
assert found, "expected explicit synchronization-only Tokio dependency"
print(f"Scheduler graph: {len(packages) - 2} dependencies beyond fixture and library; no Tokio executor/I/O/time")
