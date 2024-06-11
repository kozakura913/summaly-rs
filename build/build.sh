set -eu
source build/env.sh
ls /app/target/
cargo build --release --target ${RUST_TARGET}
cargo build --release --target ${RUST_TARGET} --example healthcheck
cp /app/target/${RUST_TARGET}/release/summaly-rs /app/summaly-rs
cp /app/target/${RUST_TARGET}/release/examples/healthcheck /app/healthcheck
