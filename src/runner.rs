use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;
use subprocess::{Exec, Redirection};
use tempfile::TempDir;

use crate::report::{Failure, Report};

static ERROR_LINE_MATCHER: Lazy<Regex> = Lazy::new(|| Regex::new(r"---- (.*) ----").unwrap());

pub struct Runner {
    pub bins: Vec<PathBuf>,
    pub record: bool,
    iter: usize,
}

impl Runner {
    pub fn new(bins: Vec<PathBuf>, record: bool) -> Self {
        Self {
            bins,
            record,
            iter: 0,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<Report> {
        self.iter += 1;
        if self.record {
            self.run_record_test_suite()
        } else {
            self.run_test_suite()
        }
    }

    fn run_record_test_suite(&self) -> anyhow::Result<Report> {
        let mut failures = Vec::new();
        let mut buf = String::new();
        for bin in self.bins.iter() {
            let temp = Rc::new(tempfile::tempdir_in(".")?);
            let cmd = Exec::cmd("rr");
            let mut out = cmd
                .arg("record")
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
