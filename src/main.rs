//  Project:   macbash
//  File:      src/main.rs
//  Purpose:   CLI entry point
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::process::ExitCode;

fn main() -> ExitCode {
    macbash::cli::run()
}
