use clap::Parser;
use std::{ops::Deref, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    long_about = "This command allows you to run your tests an arbitrary number of times to try
        to find flaky tests, return as report of the found failing tests.",
    bin_name = "cargo"
)]
pub enum Command {
    #[structopt(name = "flaky")]
    #[structopt(
        after_help = "This command allows you to run your tests an arbitrary number of times to try
        to find flaky tests, return as report of the found failing tests."
    )]
    Flaky(Args),
}

#[derive(Debug, Parser)]
#[command(bin_name = "cargo")]
pub struct Args {
    /// Whether to run the tests in release mode.
    #[arg(long)]
    pub release: bool,
    /// The number of times the tests have to be ran.
    #[arg(long, short, default_value = "100")]
    pub iter: usize,

    #[command(flatten)]
    pub rr: RrOptions,

    #[command(flatten)]
    pub test_opts: TestOptions,
}

#[derive(Debug, Parser)]
pub struct RrOptions {
    /// Whether to record the failing tests using rr. This require rr to be installed on your
    /// system.
    #[arg(long, short)]
    pub record: bool,
    /// Where to save the rr recording.
    #[arg(
        long,
        short('o'),
        requires_if("record", "true"),
        default_value = "recordings"
    )]
    pub record_out_dir: PathBuf,
    /// Enable chaos mode for rr
    #[arg(long, requires_if("record", "true"))]
    pub chaos: bool,
}

#[derive(Debug, Parser)]
pub struct TestOptions {
    #[arg(long, short)]
    pub jobs: Option<usize>,

    /// Argument to forward when building tests
    #[arg(long, short)]
    pub build_args: Vec<String>,

    /// Argument to forward when running tests
    #[arg(last = true)]
    pub extra: Vec<String>,
}

impl Deref for Command {
    type Target = Args;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Flaky(ref args) => args,
        }
    }
}
