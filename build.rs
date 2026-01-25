#![allow(unused)]
use anyhow::Result;
use clap::CommandFactory;
use std::{env, path::PathBuf};
use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder};

#[path = "src/cli/definition.rs"]
mod cli;

fn main() -> Result<()> {
    let build = BuildBuilder::all_build()?;
    let gix = GixBuilder::all_git()?;
    let cargo = CargoBuilder::all_cargo()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .add_instructions(&cargo)?
        .emit();

    Ok(())
}
