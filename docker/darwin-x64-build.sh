#!/usr/bin/env bash
# Intel macOS: ort 2.x has no prebuilt binaries — link Homebrew ONNX Runtime dynamically.
# Release is a single executable; requires `brew install onnxruntime` at runtime.
set -euo pipefail

if [[ "$(uname -m)" != "x86_64" ]]; then
	echo "error: darwin-x64-build.sh requires a native x86_64 Mac host (got $(uname -m))" >&2
	exit 1
fi

ort_lib="${ORT_LIB_PATH:-${ORT_LIB_LOCATION:-}}"
if [[ -z "${ort_lib}" ]]; then
	for prefix in /usr/local/opt/onnxruntime /opt/homebrew/opt/onnxruntime; do
		if [[ -d "${prefix}/lib" ]]; then
			ort_lib="${prefix}/lib"
			break
		fi
	done
fi

if [[ -z "${ort_lib}" || ! -f "${ort_lib}/libonnxruntime.dylib" ]]; then
	echo "error: install onnxruntime (brew install onnxruntime) or set ORT_LIB_PATH" >&2
	exit 1
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
# Always use the x86_64 layout (workflow may set CARGO_TARGET_DIR=cross-target for ARM).
export CARGO_TARGET_DIR="${repo_root}/cross-target/x86_64-apple-darwin"
export ORT_LIB_PATH="${ort_lib}"
export ORT_PREFER_DYNAMIC_LINK=1
export DYLD_FALLBACK_LIBRARY_PATH="${ort_lib}:${DYLD_FALLBACK_LIBRARY_PATH:-}"

if command -v xcrun >/dev/null 2>&1; then
	export SDKROOT="${SDKROOT:-$(xcrun --sdk macosx --show-sdk-path)}"
fi
if [[ -z "${SDKROOT}" || ! -d "${SDKROOT}" ]]; then
	export SDKROOT="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk"
fi
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.4}"
export CC="${CC:-clang}"
export CXX="${CXX:-clang++}"
export CFLAGS="${CFLAGS:--isysroot ${SDKROOT}}"
export CXXFLAGS="${CXXFLAGS:--isysroot ${SDKROOT} -I${SDKROOT}/usr/include/c++/v1}"

cd "${repo_root}"
exec cargo build --release "$@"
