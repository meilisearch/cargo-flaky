use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;
use subprocess::{Exec, Redirection};
use tempfile::TempDir;
use indicatif::ProgressBar;

use crate::args::RrOptions;
use crate::report::{Failure, Report};

static ERROR_LINE_MATCHER: Lazy<Regex> = Lazy::new(|| Regex::new(r"---- (.*) ----").unwrap());

pub struct Runner<'a> {
    pub bins: Vec<PathBuf>,
    pub rr: &'a RrOptions,
    times: usize,
}

struct RrTask<'a> {
    bin: PathBuf,
    opts: &'a RrOptions,
    iter: usize,
}

impl<'a> RrTask<'a> {
    fn new(bin: &Path, opts: &'a RrOptions) -> Self {
        Self {
            bin: bin.to_owned(),
            opts,
            iter: 0,
        }

    }

    fn cmd(&self, record_path: &Path) -> Exec {
            let mut cmd = Exec::cmd("rr");

            cmd = cmd.arg("record");

            if self.opts.chaos {
                cmd = cmd.arg("--chaos");
            }

            let mut out = cmd
                .arg("-o")
                .arg(record_path.join(format!("record_iter_{}", self.iter)))
                .arg(self.bin)
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge);
        cmd
    }
}

impl Task for RrTask<'_> {
    fn run(&mut self) -> anyhow::Result<Vec<Failure>> {
        self.iter += 1;
        let temp = Rc::new(tempfile::tempdir_in(".")?);
        let mut buf = String::new();

        let out = self.cmd(temp.path()).popen()?;

        out.stdout
            .take()
            .context("could not read from process stdout")?
            .read_to_string(&mut buf)?;

        let reader = Cursor::new(buf.as_bytes());

        let ret = parse_test_output(reader, Some(temp.clone()), &self.bin, self.iter)?;

        // check if there was an issue with rr and return it
        if !out.wait()?.success() && ret.is_empty() {
            anyhow::bail!("rr exited with failure:\n{}", buf);
        }

        Ok(ret)
    }
}

trait Task {
    fn run(&mut self) -> anyhow::Result<Vec<Failure>>;
}

impl<'a> Runner<'a> {
    pub fn new(bins: Vec<PathBuf>, rr: &'a RrOptions, times: usize) -> Self {
        Self {
            bins,
            rr,
            times,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<Report> {
        for bin in bins {
            println!("Running tests from {}", bin.display());
            let task = if self.rr.record {
                Box::new(RrTask::new(&bin,&self.rr))
            } else {
                todo!()
            };


            let progress = ProgressBar::new(self.times as u64);
            let progress = progress.wrap_iter((0..self.times).map(|_| task.run()));



        }
        todo!()
    }

    fn run_record_test_suite(&self) -> anyhow::Result<Report> {
        let mut failures = Vec::new();
        let mut buf = String::new();
        for bin in self.bins.iter() {
            let temp = Rc::new(tempfile::tempdir_in(".")?);
            let mut cmd = Exec::cmd("rr");

            cmd = cmd.arg("record");

            if self.rr.chaos {
                cmd = cmd.arg("--chaos");
            }

            let mut out = cmd
                .arg("-o")
                .arg(temp.path().join(format!("record_iter_{}", self.iter)))
                .arg(bin)
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge)
                .popen()?;

            out.stdout
                .take()
                .context("could not read from process stdout")?
                .read_to_string(&mut buf)?;

            let reader = Cursor::new(buf.as_bytes());

            let ret = parse_test_output(reader, Some(temp.clone()), &bin, self.iter)?;

            // check if there was an issue with rr and return it
            if !out.wait()?.success() && ret.is_empty() {
                anyhow::bail!("rr exited with failure:\n{}", buf);
            }

            failures.extend(ret.into_iter());

            buf.clear();
        }

        if failures.is_empty() {
            Ok(Report::Ok)
        } else {
            Ok(Report::Failures(failures))
        }
    }

    fn run_test_suite(&self) -> anyhow::Result<Report> {
        let mut failures = Vec::new();
        for bin in self.bins.iter() {
            let cmd = Exec::cmd(bin);
            let out = cmd
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge)
                .stream_stdout()?;

            let ret = parse_test_output(out, None, &bin, self.iter)?;
            failures.extend(ret.into_iter());
        }
        if failures.is_empty() {
            Ok(Report::Ok)
        } else {
            Ok(Report::Failures(failures))
        }
    }
}

fn parse_test_output(
    reader: impl Read,
    recording: Option<Rc<TempDir>>,
    bin: &Path,
    batch: usize,
) -> anyhow::Result<Vec<Failure>> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut failures = Vec::new();

    while reader.read_line(&mut line)? > 0 {
        if ERROR_LINE_MATCHER.is_match(&line) {
            let name = ERROR_LINE_MATCHER
                .captures_iter(&line)
                .next()
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .to_string();

            let mut message = String::new();
            line.clear();
            while reader.read_line(&mut line)? > 0 {
                if line.trim().is_empty() {
                    break;
                }
                message.push_str(&line);
                line.clear();
            }
            failures.push(Failure {
                name,
                batch,
                message,
                bin: bin.to_owned(),
                recording: recording.clone(),
            })
        }
        line.clear();
    }

    Ok(failures)
}
