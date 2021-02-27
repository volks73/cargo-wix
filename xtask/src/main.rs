use anyhow::Result;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum XTask {
    #[structopt(alias = "ghp")]
    Ghp,
}

fn main() -> Result<()> {
    let args = XTask::from_args();
    println!("{:?}", args);
    Ok(())
}
