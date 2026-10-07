#![cfg(feature = "zstd")]
use simple_server::zstd;
const FRAME: &[u8] = include_bytes!("fixtures/zstd-reference.bin");

#[test]
fn independent_reference_frame_and_roundtrips_without_a_runtime() {
    assert_eq!(
        zstd::decode_all(FRAME).unwrap(),
        b"weather artifact\n".repeat(100)
    );
    for level in [-3, 0, 1, 3, 19] {
        for input in [Vec::new(), vec![0, 255, 13, 10], vec![b'a'; 128 * 1024]] {
            let frame = zstd::encode_all(&input, level).unwrap();
            assert_eq!(zstd::decode_all(&frame).unwrap(), input);
        }
    }
}
#[test]
fn concatenated_and_skippable_frames_preserve_decoder_semantics() {
    let mut frames = FRAME.to_vec();
    frames.extend_from_slice(&0x184D2A50_u32.to_le_bytes());
    frames.extend_from_slice(&3_u32.to_le_bytes());
    frames.extend_from_slice(b"ski");
    frames.extend_from_slice(FRAME);
    assert_eq!(
        zstd::decode_all(&frames).unwrap(),
        b"weather artifact\n".repeat(200)
    );
}
#[test]
fn malformed_truncated_and_trailing_data_fail_without_partial_output() {
    let mut corrupted = FRAME.to_vec();
    *corrupted.last_mut().unwrap() ^= 1;
    for input in [
        corrupted,
        Vec::new(),
        b"not zstd".to_vec(),
        FRAME[..FRAME.len() - 1].to_vec(),
        [FRAME, b"garbage"].concat(),
    ] {
        assert!(zstd::decode_all(&input).is_err());
    }
}
#[test]
fn output_limits_are_exact_including_empty_and_concatenated_frames() {
    let plain = b"weather artifact\n".repeat(100);
    assert_eq!(zstd::decode_all_limited(FRAME, plain.len()).unwrap(), plain);
    assert_eq!(
        zstd::decode_all_limited(FRAME, plain.len() - 1)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    assert!(zstd::decode_all_limited(&[FRAME, FRAME].concat(), plain.len()).is_err());
    assert!(zstd::decode_all_limited(FRAME, 0).is_err());
    assert_eq!(
        zstd::decode_all_limited(&zstd::encode_all(&[], 1).unwrap(), 0).unwrap(),
        Vec::<u8>::new()
    );
}
#[test]
fn independent_threads_do_not_share_codec_state() {
    let threads: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let bytes = vec![i; 4096];
                assert_eq!(
                    zstd::decode_all(&zstd::encode_all(&bytes, 1).unwrap()).unwrap(),
                    bytes
                );
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
