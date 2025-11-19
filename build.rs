#![allow(unused)]
use clap::CommandFactory;
use std::{env, path::PathBuf};

#[path = "src/cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let cmd = cli::Cli::command().name("filessh");
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("filessh.1"), buffer)?;

    Ok(())
}
