use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARTIFACTS: &str = include_str!("artifacts.sha256");

fn sha256(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum is required to verify simple-server engine artifacts");
    assert!(
        output.status.success(),
        "sha256sum failed for {}",
        path.display()
    );
    String::from_utf8(output.stdout)
        .expect("sha256sum output is not UTF-8")
        .split_whitespace()
        .next()
        .expect("sha256sum returned no digest")
        .to_owned()
}

fn validate_elf(path: &Path, target: &str) {
    let mut header = [0_u8; 20];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .unwrap_or_else(|error| panic!("cannot read engine {}: {error}", path.display()));
    assert_eq!(
        &header[..6],
        b"\x7fELF\x02\x01",
        "engine must be a little-endian ELF64 shared library"
    );
    assert_eq!(
        u16::from_le_bytes([header[16], header[17]]),
        3,
        "engine must be an ELF shared object"
    );
    let machine = if target == "x86_64-unknown-linux-gnu" {
        62
    } else {
        183
    };
    assert_eq!(
        u16::from_le_bytes([header[18], header[19]]),
        machine,
        "engine architecture does not match TARGET={target}"
    );
}

fn enabled(name: &str) -> bool {
    env::var(name).is_ok_and(|value| matches!(value.as_str(), "1" | "true"))
}

fn obtain(target: &str, out: &Path) -> PathBuf {
    if let Some(directory) = env::var_os("SIMPLE_SERVER_ENGINE_DIR") {
        let path = PathBuf::from(directory).join("libsimple_server_engine.so");
        println!("cargo:rerun-if-changed={}", path.display());
        println!(
            "cargo:warning=using explicitly installed engine {}; release checksum pinning is bypassed",
            path.display()
        );
        return path;
    }
    let digest = ARTIFACTS.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let digest = fields.next()?;
        let platform = fields.next()?;
        (platform == target).then_some(digest)
    }).next().unwrap_or_else(|| panic!(
        "engine {VERSION} has no pinned artifact for {target}; release is not ready. For local development set SIMPLE_SERVER_ENGINE_DIR to your built engine directory"
    ));
    assert!(
        digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid engine checksum manifest"
    );
    let cache = env::var_os("SIMPLE_SERVER_ENGINE_CACHE")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("CARGO_HOME").map(|home| PathBuf::from(home).join("simple-server-engine"))
        })
        .unwrap_or_else(|| out.join("engine-cache"))
        .join(VERSION)
        .join(target)
        .join(digest);
    let library = cache.join("libsimple_server_engine.so");
    if !library.exists() {
        assert!(
            !enabled("SIMPLE_SERVER_ENGINE_OFFLINE") && !enabled("CARGO_NET_OFFLINE"),
            "engine artifact is not cached; offline mode forbids downloading. Install it with SIMPLE_SERVER_ENGINE_DIR or populate SIMPLE_SERVER_ENGINE_CACHE first"
        );
        fs::create_dir_all(&cache).expect("cannot create engine artifact cache");
        let temporary = cache.join(format!("download-{}.so", std::process::id()));
        struct Temporary(PathBuf);
        impl Drop for Temporary {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let temporary = Temporary(temporary);
        let url = format!(
            "https://github.com/lelloman/simple-server/releases/download/engine-v{VERSION}/libsimple_server_engine-{VERSION}-{target}.so"
        );
        let status = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "20",
                "--max-time",
                "300",
                "--output",
            ])
            .arg(&temporary.0)
            .arg(&url)
            .status()
            .expect("curl is required to download the prebuilt engine");
        assert!(status.success(), "engine download failed: {url}");
        assert_eq!(
            sha256(&temporary.0),
            digest,
            "downloaded engine checksum mismatch"
        );
        validate_elf(&temporary.0, target);
        fs::rename(&temporary.0, &library).expect("cannot install verified engine in cache");
    }
    assert_eq!(
        sha256(&library),
        digest,
        "cached engine checksum mismatch; remove the corrupt cache entry and retry"
    );
    println!("cargo:rerun-if-changed={}", library.display());
    library
}

fn main() {
    for name in [
        "SIMPLE_SERVER_ENGINE_DIR",
        "SIMPLE_SERVER_ENGINE_CACHE",
        "SIMPLE_SERVER_ENGINE_OFFLINE",
        "CARGO_NET_OFFLINE",
        "CARGO_HOME",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=artifacts.sha256");
    let target = env::var("TARGET").unwrap();
    assert!(
        matches!(
            target.as_str(),
            "x86_64-unknown-linux-gnu" | "aarch64-unknown-linux-gnu"
        ),
        "simple-server engine does not support target {target}"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let library = obtain(&target, &out);
    validate_elf(&library, &target);
    for name in ["libsimple_server_engine.so", "libsimple_server_engine.so.1"] {
        fs::copy(&library, out.join(name))
            .unwrap_or_else(|error| panic!("cannot install {}: {error}", library.display()));
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=dylib=simple_server_engine");
    println!("cargo:lib_dir={}", out.display());
}
