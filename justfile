default: build

build:
    cargo build

build-release:
    cargo build --release

run *ARGS:
    cargo run --bin lsc -- {{ARGS}}

test:
    cargo test --all-features

test-one TEST:
    cargo test {{TEST}} -- --nocapture

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check lint test

bench:
    cargo bench

compare:
    scripts/compare.sh

parity:
    scripts/parity.sh

install:
    cargo install --path . --bin lsc

man:
    @mkdir -p target
    ronn -r --pipe man/lsc.1.ronn > target/lsc.1

snapshot:
    cargo insta review

coverage:
    cargo llvm-cov --html

clean:
    cargo clean
    rm -rf bench/out
