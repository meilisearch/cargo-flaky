use std::{ops::Deref, path::PathBuf};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub enum Command {
    #[structopt(name = "flaky")]
    #[structopt(
        after_help = "This command allows you to run your tests an arbitrary number of times to try
        to find flaky tests, return as report of the found failing tests."
    )]
    Flaky(Args),
}

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub struct Args {
    /// Whether to run the tests in release mode.
    #[structopt(long)]
    pub release: bool,
    /// The number of times the tests have to be ran.
    #[structopt(long, short, default_value = "100")]
    pub iter: usize,
    /// If set, runs for all the iteration defined by repeat, otherwise, stops as soon as a faling
    /// test is found.
    #[structopt(long, short)]
    pub args: Option<String>,

    #[structopt(flatten)]
    pub rr: RrOptions,
}

#[derive(Debug, StructOpt)]
pub struct RrOptions {
    /// Whether to record the failing tests using rr. This require rr to be installed on your
    /// system.
    #[structopt(long, short)]
    pub record: Option<bool>,
    /// Where to save the rr recording.
    #[structopt(long, short = "o", requires_if("record", "true"), default_value = "recordings")]
    pub record_out_dir: PathBuf,
    /// Enable chaos mode for rr
    #[structopt(long, requires_if("record", "true"))]
    pub chaos: Option<bool>,
}

impl Deref for Command {
    type Target = Args;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Flaky(ref args) => args
        }
    }
}
