use anyhow::{bail, Result};
use structopt::StructOpt;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, StructOpt)]
enum XTask {
    /// Upload documentation to Github Pages (ghp) via the `gh-pages` branch.
    Ghp,
}

fn copy<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<()> {
    let mut stack = Vec::new();
    stack.push(PathBuf::from(from.as_ref()));

    let output_root = PathBuf::from(to.as_ref());
    let input_root = PathBuf::from(from.as_ref()).components().count();

    while let Some(working_path) = stack.pop() {
        println!("process: {}", &working_path.display());

        // Generate a relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        if fs::metadata(&dest).is_err() {
            println!("mkdir: {}", dest.display());
            fs::create_dir_all(&dest)?;
        }

        for entry in fs::read_dir(working_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        let dest_path = dest.join(filename);
                        println!("copy: {} -> {}", &path.display(), &dest_path.display());
                        fs::copy(&path, &dest_path)?;
                    }
                    None => {
                        println!("failed: {}", path.display());
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = XTask::from_args();
    match args {
        XTask::Ghp => {
            let cargo = env::var("CARGO")
                .map(PathBuf::from)
                .ok()
                .unwrap_or_else(|| PathBuf::from("cargo"));
            if !Command::new(cargo)
                .arg("doc")
                .arg("--no-deps")
                .status()?
                .success()
            {
                bail!("The 'cargo doc --no-deps' command failed");
            }
            if !Command::new("git")
                .arg("checkout")
                .arg("gh-pages")
                .status()?
                .success()
            {
                bail!("The 'git checkout gh-pages' command failed");
            }
            let mut target_doc_dir = PathBuf::from("target");
            target_doc_dir.push("doc");
            copy(target_doc_dir, env::current_dir()?)?;
            if !Command::new("git")
                .arg("add")
                .arg("--verbose")
                .arg("--all")
                .status()?
                .success()
            {
                bail!("The 'git add --verbose --all' command failed")
            }
            if Command::new("git")
                .arg("commit")
                .arg("--verbose")
                .arg("-m")
                .arg("Change content to match latest revision")
                .status()?
                .success()
            {
                if !Command::new("git").arg("push").status()?.success() {
                    bail!("The 'git push' command failed");
                }
            }
            if !Command::new("git")
                .arg("checkout")
                .arg("-")
                .status()?
                .success()
            {
                bail!("The 'git checkout main' command failed");
            }
        }
    }
    Ok(())
}
