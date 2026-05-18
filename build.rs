//  Project:   macbash
//  File:      build.rs
//  Purpose:   Emit build timestamp and git SHA as env vars for --version output
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::error::Error;

use vergen_gix::{BuildBuilder, Emitter, GixBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let gix = GixBuilder::default().sha(true).build()?;
    Emitter::default()
        .fail_on_error()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()?;
    Ok(())
}
