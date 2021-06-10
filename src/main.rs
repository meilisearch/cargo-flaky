mod args;
mod report;
mod runner;

use std::path::PathBuf;

use cargo::core::compiler::CompileMode;
use cargo::core::compiler::MessageFormat;
use cargo::core::Workspace;
use cargo::ops::compile;
use cargo::ops::CompileOptions;
use cargo::util::config::Config;
use cargo::util::interning::InternedString;
use runner::Runner;
use structopt::StructOpt;

use args::Command;

fn compile_tests(command: &Command) -> anyhow::Result<Vec<PathBuf>> {
    let manifest_path = std::env::current_dir()?.join("Cargo.toml");
    let config = Config::default().unwrap();

    let mut options = CompileOptions::new(&config, CompileMode::Test)?;
    options.build_config.message_format = MessageFormat::Short;

    if command.release {
        let profile = InternedString::new("release");
        options.build_config.requested_profile = profile;
    }

    let workspace = Workspace::new(&manifest_path, &config)?;
    let paths = if workspace.is_virtual() {
        let mut res = Vec::new();
        for package in workspace.members() {
            let workspace = Workspace::new(package.manifest_path(), &config)?;
            let compile = compile(&workspace, &options)?;
            let mut paths = compile.tests.into_iter().map(|c| c.path).collect();
            res.append(&mut paths)
        }
        res
    } else {
        compile(&workspace, &options)?.tests.into_iter().map(|c| c.path).collect()
    };

    Ok(paths)
}

fn main() -> anyhow::Result<()> {
    let command = Command::from_args();

    let bin_paths = compile_tests(&command)?;

    let mut runner = Runner::new(bin_paths, &command.rr, command.iter, &command.test_opts);

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
