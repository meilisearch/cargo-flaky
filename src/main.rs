mod args;
mod check;
mod report;

use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use std::io::stdout;

use structopt::StructOpt;

use args::Command;

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

    for msg in rcv {
        let _msg = msg?;

        progress.progress();
        progress.print();
    }

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
        let out = format!("\r[{:<50}] {}/{}, eta: {}",
            (0..fill.saturating_sub(1)).map(|_| '=').chain(Some('>')).collect::<String>(),
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
