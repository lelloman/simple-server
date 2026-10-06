#!/usr/bin/env python3
"""Compare equivalent source/engine HTTP builds; no engine build or downloads timed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import selectors
import shutil
import statistics
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "tests/fixtures/build_comparison"


def smoke(binary, env, revision):
    with subprocess.Popen([str(binary)], env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, text=True) as server:
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(server.stdout, selectors.EVENT_READ)
                if not selector.select(10):
                    raise RuntimeError("server did not announce its address")
                address = server.stdout.readline().strip()
            if not address.startswith("127.0.0.1:"):
                raise RuntimeError(f"invalid server address: {address!r}; exit={server.poll()}")
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

            def request(path, data=None, method=None):
                req = urllib.request.Request(f"http://{address}{path}", data=data,
                                             method=method, headers={"Content-Type": "application/json"})
                try:
                    response = opener.open(req, timeout=5)
                except urllib.error.HTTPError as error:
                    response = error
                with response:
                    return response.status, response.read()

            assert request("/health") == (200, b"ok")
            assert request("/health", method="HEAD") == (200, b"")
            status, body = request("/items/42")
            assert status == 200 and json.loads(body) == {"id": 42, "revision": revision}
            payload = {"message": "hello", "values": [1, 2, 3]}
            status, body = request("/echo", json.dumps(payload).encode())
            assert status == 200 and json.loads(body) == payload
            assert request("/missing")[0] == 404
            assert request("/shutdown", b"") == (200, b"stopping")
            assert server.wait(timeout=10) == 0
        finally:
            if server.poll() is None:
                server.kill()
            server.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="new directory for results, logs and build targets")
    parser.add_argument("--engine-dir", type=Path, required=True)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    if args.trials < 1 or args.jobs < 1:
        parser.error("trials and jobs must be positive")
    engine = args.engine_dir.resolve() / "libsimple_server_engine.so"
    if not engine.is_file():
        parser.error("engine-dir must contain libsimple_server_engine.so")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ)
    # Disable compiler wrappers/shared artifacts; preserve the installed toolchain.
    for key in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    env.update(RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="", SIMPLE_SERVER_ENGINE_DIR=str(engine.parent), CARGO_INCREMENTAL="1", CARGO_NET_OFFLINE="true")
    runtime_lib = output / "lib"
    runtime_lib.mkdir()
    (runtime_lib / "libsimple_server_engine.so.1").symlink_to(engine)
    env["LD_LIBRARY_PATH"] = str(runtime_lib) + (":" + env["LD_LIBRARY_PATH"] if env.get("LD_LIBRARY_PATH") else "")
    result = {
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "platform": platform.platform(), "cpu_count": os.cpu_count(),
        "cpuinfo": Path("/proc/cpuinfo").read_text().split("\n\n")[0],
        "jobs": args.jobs, "profile": "dev", "incremental": True,
        "fixture_sha256": {str(path.relative_to(FIXTURE)): hashlib.sha256(path.read_bytes()).hexdigest()
                           for path in sorted(FIXTURE.rglob("*")) if path.is_file() and "target" not in path.parts},
        "engine": str(engine), "engine_bytes": engine.stat().st_size,
        "engine_sha256": hashlib.sha256(engine.read_bytes()).hexdigest(),
        "scope": "fresh Cargo targets, warm OS/source cache; engine prebuild/download excluded; no runtime benchmark",
        "rows": [],
    }
    for trial in range(args.trials):
        order = ("source", "engine") if trial % 2 == 0 else ("engine", "source")
        for backend in order:
            directory = output / f"trial-{trial + 1}-{backend}"
            consumer = directory / "consumer"
            shutil.copytree(FIXTURE, consumer, ignore=shutil.ignore_patterns("target"))
            manifest = consumer / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace('path = "../../.."', f'path = "{ROOT}"'))
            target = directory / "target"
            command = ["cargo", "build", "--offline", "--locked", "--manifest-path", str(manifest),
                       "--no-default-features", "--features", backend, "--target-dir", str(target), "-j", str(args.jobs)]
            for phase in ("clean", "noop", "edit"):
                if phase == "edit":
                    main_rs = consumer / "src/main.rs"
                    main_rs.write_text(main_rs.read_text().replace("const REVISION: u32 = 1;", "const REVISION: u32 = 2;"))
                log = directory / f"{phase}.log"
                started = time.perf_counter()
                with log.open("w") as stream:
                    subprocess.run(command, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
                elapsed = time.perf_counter() - started
                binary = target / "debug/simple-server-build-comparison"
                smoke(binary, env, 2 if phase == "edit" else 1)
                row = {"trial": trial + 1, "backend": backend, "phase": phase,
                       "seconds": elapsed, "binary_bytes": binary.stat().st_size, "smoke": "passed"}
                result["rows"].append(row)
                (output / "results.json").write_text(json.dumps(result, indent=2) + "\n")
                print(f"{trial + 1} {backend:6} {phase:5}: {elapsed:.3f}s", flush=True)
            graph = subprocess.check_output([
                "cargo", "tree", "--offline", "--locked", "--manifest-path", str(manifest),
                "--no-default-features", "--features", backend, "--edges", "normal,build", "--prefix", "none",
            ], env=env, text=True)
            (directory / "graph.txt").write_text(graph)
            packages = {tuple(line.split()[:2]) for line in graph.splitlines() if line.strip()}
            result.setdefault("dependency_packages_excluding_consumer_and_library", {})[backend] = len(packages) - 2
    result["medians_seconds"] = {
        backend: {phase: statistics.median(row["seconds"] for row in result["rows"]
                  if row["backend"] == backend and row["phase"] == phase)
                  for phase in ("clean", "noop", "edit")}
        for backend in ("source", "engine")
    }
    (output / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["medians_seconds"], indent=2))


if __name__ == "__main__":
    main()
