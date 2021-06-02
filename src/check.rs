use once_cell::sync::Lazy;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use subprocess::{Exec, Redirection};

use crate::report::{Failure, Report};

static ERROR_LINE_MATCHER: Lazy<Regex> = Lazy::new(|| Regex::new(r"---- (.*) ----").unwrap());

pub struct Check {
    pub iters: usize,
    pub release: bool,
    pub snd: mpsc::Sender<anyhow::Result<Report>>,
    pub failed: bool,
}

impl Check {
    pub fn run(mut self) {
        for _ in 0..self.iters {
            let result = self.run_test_suite();
            let _ = self.snd.send(result);
        }
    }

    fn run_test_suite(&mut self) -> anyhow::Result<Report> {
        let mut cmd = Exec::cmd("cargo");
        cmd = cmd.arg("test");
        if self.release {
            cmd = cmd.arg("--release");
        }

        let out = cmd
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge)
            .stream_stdout()?;

        let mut reader = BufReader::new(out);
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
                failures.push(Failure { name, message })
            }
            line.clear();
        }

        if failures.is_empty() {
            Ok(Report::Ok)
        } else {
            Ok(Report::Failures(failures))
        }
    }
}
