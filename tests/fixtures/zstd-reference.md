# Zstd reference frame

`zstd-reference.bin` is generated from `b"weather artifact\n" * 100` using
`zstd -q -c -3` (CLI 1.5.5). It contains only synthetic test data.
The engine codec tests decode it independently of their encoder implementation.
