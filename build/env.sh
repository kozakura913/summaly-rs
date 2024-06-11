set -eu
if [ ${TARGETARCH} = "amd64" ]; then
	export MUSL_NAME="x86_64-linux-musl-cross"
	export CC=x86_64-linux-musl-gcc
	export RUST_TARGET="x86_64-unknown-linux-musl"
	export RUSTFLAGS="-C linker=${CC}"
elif [ ${TARGETARCH} = "386" ]; then
	export MUSL_NAME="i686-linux-musl-cross"
	export CC=i686-linux-musl-gcc
	export RUST_TARGET="i686-unknown-linux-musl"
	#現時点ではringがsse2を必須としている
	#https://github.com/briansmith/ring/blob/main/src/cpu/intel.rs#L23
	#https://github.com/briansmith/ring/issues/1793#issuecomment-1793243725
	#https://github.com/briansmith/ring/issues/1832
	#https://github.com/briansmith/ring/issues/1833.
	export RUSTFLAGS="-C target-feature=+sse -C target-feature=+sse2 -C linker=${CC}"
elif [ ${TARGETARCH} = "arm64" ]; then
	export MUSL_NAME="aarch64-linux-musl-cross"
	export CC=aarch64-linux-musl-gcc
	export RUST_TARGET="aarch64-unknown-linux-musl"
	export RUSTFLAGS="-C linker=${CC}"
elif [ ${TARGETARCH} = "arm" ]; then
	if [ ${TARGETVARIANT} = "v7" ]; then
		export MUSL_NAME="armv7l-linux-musleabihf-cross"
		export CC=armv7l-linux-musleabihf-gcc
		export RUSTFLAGS="-C linker=${CC}"
		export RUST_TARGET="armv7-unknown-linux-musleabihf"
	elif [ ${TARGETVARIANT} = "v6" ]; then
		export MUSL_NAME="armv6-linux-musleabihf-cross"
		export CC=armv6-linux-musleabihf-gcc
		export RUSTFLAGS="-C linker=${CC}"
		export RUST_TARGET="arm-unknown-linux-musleabihf"
	else
		echo "NO Support Target ${TARGETARCH}/${TARGETVARIANT}"
		exit 1
	fi
else
	echo "NO Support Target ${TARGETARCH}/${TARGETVARIANT}"
	exit 1
fi
export PATH="/${MUSL_NAME}/bin:${PATH}"
