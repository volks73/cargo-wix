use anyhow::{bail, Result};
use structopt::StructOpt;

use std::env;
use std::process::Command;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
enum XTask {
    Ghp,
}

fn main() -> Result<()> {
    let args = XTask::from_args();
    match args {
        XTask::Ghp => {
            let cargo = env::var("CARGO")
                .map(PathBuf::from)
                .ok()
                .unwrap_or_else(|| PathBuf::from("cargo"));
            let status = Command::new(cargo)
                .arg("doc")
                .arg("--no-deps")
                .status()?;
            if !status.success() {
                bail!("The 'cargo doc' command failed");
            }
        }
    }
    Ok(())
}
