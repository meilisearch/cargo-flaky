mod args;
mod check;
mod report;

use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use std::io::stdout;

use structopt::StructOpt;

use args::Command;
use report::Failure;

fn main() -> anyhow::Result<()> {
    let command = Command::from_args();

    let (snd, rcv) = mpsc::channel();
    let check = check::Check {
        iters: command.repeat,
        release: command.release,
        snd,
        failed: false,
    };

    std::thread::spawn(|| check.run());

    let mut progress = Progress::new(command.repeat);
    let mut results = HashMap::new();

    for msg in rcv {
        let msg = msg?;

        progress.progress();
        progress.print();

        match msg {
            report::Report::Failures(failures) => {
                for failure in failures  {
                    match results.entry(failure.name.clone()) {
                        Vacant(entry) => {
                            entry.insert((failure, 1));
                        },
                        Occupied(mut entry) => {
                            entry.get_mut().1 += 1;
                        },
                    }
                }
            },
            report::Report::Ok => (),
        }
    }
    println!();

    print_report(results, command.repeat);

    Ok(())
}

fn print_report(results: HashMap<String, (Failure, usize)>, iters: usize) {
    if results.is_empty() {
        return println!("found no failing tests.");
    }

    println!("--- Found {} failing test ---\n", results.len());
    for (_, (failure, count)) in results {
        println!("test: {}, {}/{} ({}%)", failure.name, count, iters, (count as f64) * 100.0 / (iters as f64));
        println!("message:\n{}", failure.message);
        println!("\n--------------------------------\n")
    }

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
        let out = format!("\r[{:<50}] {}/{}, eta: {}",
            (0..fill.saturating_sub(1)).map(|_| '=').chain(Some('>')).take(50).collect::<String>(),
            self.current,
            self.total,
            self.eta.map(|d| format!("{} secs", d.as_secs())).unwrap_or_else(|| String::from("Unknown")),
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
