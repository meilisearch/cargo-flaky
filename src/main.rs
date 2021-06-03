mod args;
mod report;
mod runner;

use cargo::core::compiler::CompileMode;
use cargo::core::Workspace;
use cargo::ops::compile;
use cargo::ops::CompileOptions;
use cargo::util::config::Config;
use chrono::Utc;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::stdout;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use cargo::util::interning::InternedString;
use report::Report;
use runner::Runner;
use structopt::StructOpt;

use args::Command;
use report::Failure;

fn compile_tests(command: &Command) -> anyhow::Result<Vec<PathBuf>> {
    let manifest_path = std::env::current_dir()?.join("Cargo.toml");
    let config = Config::default().unwrap();
    let workspace = Workspace::new(&manifest_path, &config)?;

    let mut options = CompileOptions::new(&config, CompileMode::Test)?;

    if command.release {
        let profile = InternedString::new("release");
        options.build_config.requested_profile = profile;
    }

    let compilation = compile(&workspace, &options)?;

    let paths = compilation.tests.into_iter().map(|c| c.path).collect();
    Ok(paths)
}

struct FailureReport {
    /// Batches where this test failed
    batches: HashSet<usize>,
    /// Name of the failed test
    name: String,
    /// Path to the bin that triggered the test failure
    bin: PathBuf,
    recording: Option<PathBuf>,
    message: String,
}

struct Reports {
    record_path: PathBuf,
    reports: HashMap<String, FailureReport>,
    seen_batches: HashSet<usize>,
    total_iters: usize,
}

impl Reports {
    fn new(record_path: &Path, total_iters: usize) -> Self {
        let reports = HashMap::new();
        let seen_batches = HashSet::new();
        Self {
            reports,
            record_path: record_path.into(),
            seen_batches,
            total_iters,
        }
    }

    /// Registers a failure.
    fn register(&mut self, failure: Failure) -> anyhow::Result<()> {
        match self.reports.entry(failure.name.clone()) {
            Entry::Vacant(entry) => {
                let mut batches = HashSet::new();
                batches.insert(failure.batch);

                let recording = match failure.recording {
                    Some(ref tmp) if !self.seen_batches.contains(&failure.batch) => {
                        let name = format!("record_iter_{}", failure.batch);
                        let src = tmp.path().join(&name);
                        let dst = self.record_path.join(&name);
                        std::fs::rename(src, &dst)?;
                        Some(dst)
                    }
                    // The recording is moved the first time, so we subsequently only refer to it.
                    Some(_) => {
                        let name = format!("record_iter_{}", failure.batch);
                        let dst = self.record_path.join(&name);
                        Some(dst)
                    }
                    None => None,
                };

                self.seen_batches.insert(failure.batch);

                let report = FailureReport {
                    batches,
                    name: failure.name,
                    bin: failure.bin,
                    recording,
                    message: failure.message,
                };

                entry.insert(report);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().batches.insert(failure.batch);
            }
        }
        Ok(())
    }

    fn show(&self) {
        if self.reports.is_empty() {
            return println!("Found no failing tests.");
        }

        println!("--- Found {} failing test ---\n", self.reports.len());
        for (_, report) in self.reports.iter() {
            println!(
                "test: {}, {}/{} ({}%)",
                report.name,
                report.batches.len(),
                self.total_iters,
                (report.batches.len() as f64) * 100.0 / (self.total_iters as f64)
            );
            println!("Test binary: {}", report.bin.display());
            println!("message:\n{}", report.message);
            if let Some(ref recording) = report.recording {
                println!("recording available in : {}", recording.display());
            }
            println!("--------------------------------")
        }
    }
}

fn main() -> anyhow::Result<()> {
    let command = Command::from_args();

    let bin_paths = compile_tests(&command)?;
    let mut suite = Runner::new(bin_paths, command.record);

    let record_out_dir = match command.record_out_dir {
        Some(ref command) => command.clone(),
        None => PathBuf::from(format!("recording_{}", Utc::now().format("%Y%m%d%H%M%S"))),
    };

    let mut progress = Progress::new(command.iter);
    let mut reports = Reports::new(&record_out_dir, command.iter);

    if command.record {
        std::fs::create_dir_all(&record_out_dir)?;
    }

    for _ in 0..command.iter {
        progress.progress();
        progress.print();

        if let Report::Failures(failures) = suite.run()? {
            for failure in failures {
                reports.register(failure)?;
            }
        }
    }

    println!();

    reports.show();

    Ok(())
}

struct Progress {
    start: Instant,
    total: usize,
    current: usize,
    eta: Option<Duration>,
}

impl Progress {
    fn new(total: usize) -> Self {
        Self {
            start: Instant::now(),
            total,
            current: 0,
            eta: None,
        }
    }

    fn progress(&mut self) {
        self.current += 1;
        let elapsed = self.start.elapsed().as_secs() as usize;
        if elapsed > 0 {
            let eta = (elapsed * (self.total - self.current)) / self.current;
            self.eta = Some(Duration::from_secs(eta as u64));
        }
    }

    fn print(&self) {
        let fill = self.current * 50 / self.total;
        let out = format!(
            "\r[{:<50}] {}/{}, eta: {}",
            (0..fill.saturating_sub(1))
                .map(|_| '=')
                .chain(Some('>'))
                .take(50)
                .collect::<String>(),
            self.current,
            self.total,
            self.eta
                .map(|d| format!("{} secs", d.as_secs()))
                .unwrap_or_else(|| String::from("Unknown")),
        );
        let mut stdout = stdout();
        stdout.write_all(out.as_bytes()).unwrap();
        stdout.flush().unwrap();
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        panic!()
    }
}
