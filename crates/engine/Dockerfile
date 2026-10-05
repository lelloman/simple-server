# Build natively on each requested platform, using BuildKit/QEMU when needed.
# Publication is deliberately separate from building this image.
FROM rust:1.96-bookworm@sha256:a339861ae23e9abb272cea45dfafde21760d2ce6577a70f8a926153677902663 AS build
WORKDIR /src
COPY crates/engine-abi crates/engine-abi
COPY crates/engine crates/engine
RUN cargo build --locked --release --manifest-path crates/engine/Cargo.toml
RUN mkdir /artifact && \
    cp crates/engine/target/release/libsimple_server_engine.so /artifact/ && \
    rustc -Vv > /artifact/toolchain.txt && \
    readelf --version-info /artifact/libsimple_server_engine.so > /artifact/symbol-versions.txt && \
    readelf -d /artifact/libsimple_server_engine.so > /artifact/dynamic-section.txt && \
    sha256sum /artifact/libsimple_server_engine.so > /artifact/SHA256SUMS
FROM scratch AS artifact
COPY --from=build /artifact/ /
