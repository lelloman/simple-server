#![cfg(feature = "hashing")]
use simple_server::hashing::Sha256;
use std::io::{BufWriter, Write};

#[test]
fn standard_vectors_without_a_runtime() {
    for (input, expected) in [
        (
            Vec::new(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            b"abc".to_vec(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            vec![b'a'; 1_000_000],
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
        ),
    ] {
        let digest = Sha256::try_digest(&input).unwrap();
        assert_eq!(format!("{digest:x}"), expected);
        let bytes: [u8; 32] = digest.into();
        assert_eq!(bytes.as_slice(), digest.as_ref());
        let mut state = Sha256::try_new().unwrap();
        for chunk in input.chunks(7) {
            state.try_update(chunk).unwrap();
        }
        assert_eq!(state.try_finalize().unwrap(), digest);
    }
}
#[test]
fn chunk_boundaries_and_writer_match_independent_sha256sum() {
    for size in [0, 1, 55, 56, 63, 64, 65, 8191, 8192, 8193, 32769] {
        let input: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), &input).unwrap();
        let result = std::process::Command::new("sha256sum")
            .arg(file.path())
            .output()
            .unwrap();
        assert!(result.status.success());
        let expected = String::from_utf8(result.stdout).unwrap()[..64].to_owned();
        assert_eq!(format!("{:x}", Sha256::digest(&input)), expected);
        for chunk in [1, 8, 4097, 8192, 20000] {
            let mut state = Sha256::new();
            for part in input.chunks(chunk) {
                state.update(part);
            }
            state.try_update([]).unwrap();
            state.flush().unwrap();
            state.update(b"");
            assert_eq!(format!("{:x}", state.finalize()), expected);
        }
        let mut state = Sha256::new();
        {
            let mut writer = BufWriter::new(&mut state);
            writer.write_all(&input).unwrap();
            writer.flush().unwrap();
        }
        assert_eq!(format!("{:x}", state.finalize()), expected);
    }
}
#[test]
fn independent_states_threads_and_abandoned_hashers() {
    let threads: Vec<_> = (0..8)
        .map(|byte| {
            std::thread::spawn(move || {
                let bytes = vec![byte; 20000];
                for _ in 0..32 {
                    let mut abandoned = Sha256::new();
                    abandoned.update(&bytes);
                    drop(abandoned);
                    let mut state = Sha256::new();
                    state.update(&bytes[..13]);
                    std::thread::yield_now();
                    state.update(&bytes[13..]);
                    assert_eq!(state.finalize(), Sha256::digest(&bytes));
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
