pub struct Progress {
    start: Instant,
    times: usize,
    current: usize,
    eta: Option<Duration>,
}

pub trait Progressable

impl Progress {
    fn from_fn(times: usize; f: FnMut() -> anyhow::Result<()>) -> Self {
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
