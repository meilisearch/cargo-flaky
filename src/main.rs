mod args;
mod report;
mod runner;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use args::Command;
use clap::Parser as _;
use once_cell::sync::Lazy;
use runner::Runner;
use serde_json::Value;
use subprocess::{Exec, Redirection};

pub static SHOULD_EXIT: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

fn compile_tests(command: &Command) -> anyhow::Result<Vec<PathBuf>> {
    let mut cmd = Exec::cmd("cargo");
    cmd = cmd.args(&["build", "--tests", "--message-format", "json"]);

    if command.release {
        cmd = cmd.arg("--release");
    }

    let mut out = cmd.stdout(Redirection::Pipe).popen()?;

    let stdout = out.stdout.take().context("could not read from stdout")?;
    let mut reader = BufReader::new(stdout);
    let mut buf = String::new();
    let mut bins = Vec::new();
    while reader.read_line(&mut buf)? > 0 {
        if let Some('{') = buf.chars().next() {
            let json: Value = serde_json::from_str(&buf)?;
            if let Some(reason) = json.get("reason") {
                if reason == "compiler-artifact"
                    && json["profile"]["test"].as_bool().unwrap_or(false)
                {
                    for path in json["filenames"]
                        .as_array()
                        .context("invalid json in cargo log")?
                    {
                        bins.push(PathBuf::from(path.as_str().unwrap()));
                    }
                }
            }
        } else {
            print!("{}", buf);
        }
        buf.clear();
    }

    Ok(bins)
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(move || {
        SHOULD_EXIT.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let command = Command::parse();

    let bin_paths = compile_tests(&command)?;

    let mut runner = Runner::new(bin_paths, &command.rr, command.iter, &command.test_opts);
    let reports = runner.run()?;
    println!("{}", reports);

    if reports.failed_tests() > 0 {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        panic!()
    }
}
