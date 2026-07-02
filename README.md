# Rust Build Demo

The repository demonstrates an optimized vs. unoptimized Rust CI job.

| Unoptimized baseline                                     | Optimized                          |
|----------------------------------------------------------|------------------------------------|
| default GNU ld                                           | mold linker via RUSTFLAGS          |
| no compiler cache, full rebuild every run                | sccache with GHA backend           |
| no dependency cache, dependencies recompile every run    | Swatinem/rust-cache                |
| plain cargo test                                         | nextest via prebuilt binary        |
| cargo install cross compiled from source every run       | cargo-zigbuild via prebuilt binary |
| cross builds inside Docker containers it must pull first | Zig cross-linking on the host      |
