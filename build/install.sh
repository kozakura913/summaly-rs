set -eu
source build/env.sh
rustup target add ${RUST_TARGET}
curl -sSL https://musl.cc/${MUSL_NAME}.tgz | tar -zxf - -C /
