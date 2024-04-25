set -eu
source build/env.sh
ls /app/target/
cargo build --release --target ${RUST_TARGET}
cp /app/target/${RUST_TARGET}/release/summaly-rs /app/summaly-rs
