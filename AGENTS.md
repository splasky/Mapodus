# Project Guidelines

## Code Generation

Use `@Qwen3.6-codegen` for writing, refactoring, or reviewing Rust code in this project. Do not use the default model for code generation tasks.

## Project Context

- **Language**: Rust (nightly 1.96.0-nightly, edition 2024)
- **Toolchain**: managed via rustup, installed at ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu
- **Build**: `cargo build`, `cargo test`, `cargo build --release`
- **Project type**: CLI binary (`google-maps-to-umap`)
- **Dependencies**: clap, serde, serde_json, csv, geojson, reqwest, tokio, anyhow, chrono, uuid
- **Key modules**: src/{main,cli,error}.rs, google/mod.rs, convert/mod.rs, umap/{mod,auth,upload}.rs

## Commit Conventions

- **English only**: All commit messages must be written in English.
- **Descriptive body**: Beyond the subject line, clearly explain the reason for the change to aid future debugging and traceability.
- **One change per commit**: Each commit should represent a single logical feature or fix — do not bundle unrelated changes together.
