use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

use anyhow::Context;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use once_cell::sync::Lazy;
use regex::Regex;
use subprocess::{Exec, Redirection};
use tempfile::TempDir;

use crate::args::RrOptions;
use crate::args::TestOptions;
use crate::report::Failure;

static ERROR_LINE_MATCHER: Lazy<Regex> = Lazy::new(|| Regex::new(r"---- (.*) ----").unwrap());

struct FailureReport {
    /// Batches where this test failed
    occurences: usize,
    /// Name of the failed test
    name: String,
    /// Path to the bin that triggered the test failure
    bin: PathBuf,
    recording: Option<PathBuf>,
    message: String,
}

struct Report {
    failures: Vec<Failure>,
    recording: Option<TempDir>,
}

pub struct Reports {
    reports: HashMap<String, FailureReport>,
    total_iters: usize,
}

impl fmt::Display for Reports {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "====== FOUND {} FAILING TESTS ======",
            self.reports.len()
        )?;
        for (_, report) in self.reports.iter() {
            writeln!(f, "test: {}", report.name)?;
            writeln!(
                f,
                "failures: {}/{} ({}%)",
                report.occurences,
                self.total_iters,
                (report.occurences as f64 / self.total_iters as f64) * 100.0
            )?;
            writeln!(f, "path: {}", report.bin.display())?;
            if let Some(ref path) = report.recording {
                writeln!(f, "recording available at: {}", path.display())?;
            }
            writeln!(f, "message:\n{}", report.message)?;
            writeln!(f, "\n------------------------------------")?;
        }
        Ok(())
    }
}

pub struct Runner<'a> {
    pub bins: Vec<PathBuf>,
    pub rr: &'a RrOptions,
    times: usize,
    test_opts: &'a TestOptions,
}

struct RrTask<'a> {
    bin: PathBuf,
    opts: &'a RrOptions,
    test_opts: &'a TestOptions,
    iter: usize,
}

struct TestTask<'a> {
    bin: PathBuf,
    iter: usize,
    test_opts: &'a TestOptions,
}

impl<'a> TestTask<'a> {
    fn new(bin: &Path, test_opts: &'a TestOptions) -> Self {
        Self {
            bin: bin.to_owned(),
            iter: 0,
            test_opts,
        }
    }
}

impl Task for TestTask<'_> {
    fn run(&mut self) -> anyhow::Result<Report> {
        self.iter += 1;

        let mut buf = String::new();

        let test_threads = self.test_opts.jobs.unwrap_or_else(|| num_cpus::get()).to_string();
        let cmd = Exec::cmd(&self.bin)
            .args(&["--test-threads", &test_threads]);

        let mut out = cmd
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge)
            .popen()?;

        out.stdout
            .take()
            .context("could not read from process stdout")?
            .read_to_string(&mut buf)?;

        let reader = Cursor::new(buf.as_bytes());

        let failures = parse_test_output(reader)?;

        // check if there was an issue with rr and return it
        if !out.wait()?.success() && failures.is_empty() {
            anyhow::bail!("Unexpected test error:\n{}", buf);
        }

        let report = Report {
            failures,
            recording: None,
        };

        Ok(report)
    }
}

impl<'a> RrTask<'a> {
    fn new(bin: &Path, opts: &'a RrOptions, test_opts: &'a TestOptions) -> Self {
        Self {
            bin: bin.to_owned(),
            opts,
            iter: 0,
            test_opts,
        }
    }

    fn cmd(&self, record_path: &Path) -> Exec {
        let mut cmd = Exec::cmd("rr");

        cmd = cmd.arg("record");

        if let Some(true) = self.opts.chaos {
            cmd = cmd.arg("--chaos");
        }

        let test_threads = self.test_opts.jobs.unwrap_or_else(|| num_cpus::get()).to_string();

        cmd = cmd
            .arg("-o")
            .arg(record_path.join(format!("record_iter_{}", self.iter)))
            .arg(&self.bin)
            .args(&["--test-threads", &test_threads]);

        cmd = cmd
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge);

        cmd
    }
}

impl Task for RrTask<'_> {
    fn run(&mut self) -> anyhow::Result<Report> {
        self.iter += 1;
        let temp = tempfile::tempdir_in(".")?;
        let mut buf = String::new();

        let mut out = self.cmd(temp.path()).popen()?;

        out.stdout
            .take()
            .context("could not read from process stdout")?
            .read_to_string(&mut buf)?;

        let reader = Cursor::new(buf.as_bytes());

        let failures = parse_test_output(reader)?;

        // check if there was an issue with rr and return it
        if !out.wait()?.success() && failures.is_empty() {
            anyhow::bail!("rr exited with failure:\n{}", buf);
        }

        let report = Report {
            failures,
            recording: Some(temp),
        };

        Ok(report)
    }
}

trait Task {
    fn run(&mut self) -> anyhow::Result<Report>;
}

impl<'a> Runner<'a> {
    pub fn new(bins: Vec<PathBuf>, rr: &'a RrOptions, times: usize, test_opts: &'a TestOptions) -> Self {
        Self { bins, rr, times, test_opts }
    }

    pub fn run(&mut self) -> anyhow::Result<Reports> {
        let mut reports = HashMap::new();

        for bin in self.bins.iter() {
            println!("Running tests from {}", bin.display());
            let mut task: Box<dyn Task> = if let Some(true) = self.rr.record {
                Box::new(RrTask::new(&bin, &self.rr, &self.test_opts))
            } else {
                Box::new(TestTask::new(&bin, &self.test_opts))
            };

            let bar = ProgressBar::new(self.times as u64);
            bar.set_style(ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} eta: {eta}")
                .progress_chars("##-"));

            bar.tick();

            for i in 0..self.times {
                let mut dst_recordings = None;

                let report = task.run()?;
                if !report.failures.is_empty() {
                    for failure in report.failures.iter() {

                        if !reports.contains_key(&failure.name) {
                            bar.println(format!("Test failed: {}", failure.name));
                        }

                        match reports.entry(failure.name.clone()) {
                            Entry::Vacant(entry) => {
                                dst_recordings = match report.recording {
                                    Some(_) => Some(self.recordings_path(bin, i)?),
                                    None => None,
                                };

                                let report = FailureReport {
                                    occurences: 1,
                                    name: failure.name.clone(),
                                    bin: bin.clone(),
                                    recording: dst_recordings.clone(),
                                    message: failure.message.clone(),
                                };
                                entry.insert(report);
                            }
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().occurences += 1;
                            }
                        }
                    }
                }

                if let Some((src, dst)) = report.recording.zip(dst_recordings) {
                    std::fs::rename(src, dst)?;
                }

                bar.inc(1);
            }
        }

        Ok(Reports {
            reports,
            total_iters: self.times,
        })
    }

    fn recordings_path(&self, bin: &Path, iter: usize) -> anyhow::Result<PathBuf> {
        let filename = bin
            .file_name()
            .context("not a valid rr recordign destination")?;
        let out_dir = self.rr.record_out_dir.join(filename);
        std::fs::create_dir_all(&out_dir)?;
        Ok(out_dir.join(format!("record_iter_{}", iter)))
    }
}

fn parse_test_output(reader: impl Read) -> anyhow::Result<Vec<Failure>> {
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
            failures.push(Failure { name, message })
        }
        line.clear();
    }

    Ok(failures)
}
