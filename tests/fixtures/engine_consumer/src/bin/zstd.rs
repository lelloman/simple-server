fn main() {
    let encoded = simple_server::zstd::encode_all(b"standalone codec", 3).unwrap();
    assert_eq!(
        simple_server::zstd::decode_all(&encoded).unwrap(),
        b"standalone codec"
    );
    println!("engine Zstd consumer passed without a runtime");
}
