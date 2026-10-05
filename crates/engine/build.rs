fn main() {
    println!("cargo:rustc-link-arg-cdylib=-Wl,-soname,libsimple_server_engine.so.1");
}
