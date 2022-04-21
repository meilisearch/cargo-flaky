# cargo flaky

Cargo flaky extends cargo to help you find flaky tests in you test suite, and help you solve them.

## Motivation

Flaky tests are notoriously annoying and difficult to debug, because of their very nature: they seem to happen randomly. The idea behind cargo edit is to run you test a great number of times to try to trigger failure of those flaky tests. But this is not sufficient, because knowing that the bug exists is rarely sufficient to debug it, cargo flaky also allows you to record the failing tests using [rr](https://rr-project.org/), so you can play back the execution of your tests when they failed. Noice.

## Instalation

from source:
```bash
git clone https://github.com/meilisearch/cargo-flaky.github
cd cargo flaky
cargo install --path .
```

## Usage

Right now, cargo flaky doesn't support passing custom arguments to the test suite.

You can run all your tests an arbitrary number of times:

```bash
cargo flaky -i 30 # run your test suite 30 times and collect all failures
```

And you can record your failing test (require `rr` to be installed)
```bash
cargo flaky -i 30 -r # run your tests suite 30 time and record failures
```

outputs:

```
--- Found 1 failing test ---

test: test::test stdout, 10/10 (100%)
Test binary: /home/mpostma/Documents/code/rust/cargo-flaky/target/debug/deps/cargo_flaky-2ac6d6022fd
27a8d
message:
thread 'main' panicked at 'explicit panic', src/main.rs:231:9
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

recording available in : recording_20210603213043/record_iter_1
--------------------------------
```

You can now run `rr replay recording_20210603213043/record_iter_1` to open a `gdb` session and debug a recording of the failing test instance.

You can get more usage information by typing `cago flaky --help`.

## Disclaimer

This is still a work in progress, and lacks many features. Do not hesitate to open a PR/issue if you're missing something.

## License

MIT
