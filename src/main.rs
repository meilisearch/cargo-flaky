mod args;
mod report;
mod runner;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use cargo::core::compiler::CompileMode;
use cargo::core::Workspace;
use cargo::core::compiler::MessageFormat;
use cargo::ops::compile;
use cargo::ops::CompileOptions;
use cargo::util::config::Config;
use cargo::util::interning::InternedString;
use runner::Runner;
use structopt::StructOpt;
use args::Command;
use once_cell::sync::Lazy;

pub static SHOULD_EXIT: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

fn compile_tests(command: &Command) -> anyhow::Result<Vec<PathBuf>> {
    let manifest_path = std::env::current_dir()?.join("Cargo.toml");
    let config = Config::default().unwrap();
    let workspace = Workspace::new(&manifest_path, &config)?;

    let mut options = CompileOptions::new(&config, CompileMode::Test)?;
    options.build_config.message_format = MessageFormat::Human;

    if command.release {
        let profile = InternedString::new("release");
        options.build_config.requested_profile = profile;
    }

    let compilation = compile(&workspace, &options)?;

    let paths = compilation.tests.into_iter().map(|c| c.path).collect();
    Ok(paths)
}

fn main() -> anyhow::Result<()> {

    ctrlc::set_handler(move || {
        SHOULD_EXIT.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let command = Command::from_args();

    let bin_paths = compile_tests(&command)?;

    let mut runner = Runner::new(bin_paths , &command.rr, command.iter, &command.test_opts);

    let reports = runner.run()?;

    println!("{}", reports);

    Ok(())
}


#[cfg(test)]
mod test {
    #[test]
    fn test() {
        std::thread::sleep_ms(500);
        panic!()
    }
}
