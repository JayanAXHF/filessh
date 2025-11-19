#![allow(unused)]
use anyhow::Result;
use clap::CommandFactory;
use std::{env, path::PathBuf};
use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder};

#[path = "src/cli/definition.rs"]
mod cli;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let cmd = cli::Cli::command().name("filessh");
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("filessh.1"), buffer)?;

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
