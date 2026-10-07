use simple_server::hashing::Sha256;
fn main() {
    let mut state = Sha256::new();
    state.update(b"a");
    state.update(b"bc");
    assert_eq!(state.finalize(), Sha256::digest(b"abc"));
    println!("native SHA-256 consumer passed");
}
