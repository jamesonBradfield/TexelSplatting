#!/bin/bash
# Exit on error
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PROFILE="${1:-debug}"
TARGET_DIR="target/${PROFILE}"

echo "=== Building texel_splatting Godot Plugin ($PROFILE) ==="

mkdir -p bin

# Build the Rust library
if [ "$PROFILE" = "release" ]; then
    cargo build --manifest-path .rust/Cargo.toml --release
else
    cargo build --manifest-path .rust/Cargo.toml
fi

# Clean up old lock files
rm -f bin/~*.old 2>/dev/null || true

# Safe Swap Logic
DLL_NAME="texel_splatting"
OS_EXT=".so"
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS_EXT=".dylib"
    DLL_NAME="libtexel_splatting"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    OS_EXT=".dll"
else
    DLL_NAME="libtexel_splatting"
fi

if [ -f "bin/${DLL_NAME}${OS_EXT}" ]; then
    mv -f "bin/${DLL_NAME}${OS_EXT}" "bin/~${DLL_NAME}_$(date +%s).old" 2>/dev/null || true
fi

cp "${TARGET_DIR}/${DLL_NAME}${OS_EXT}" "bin/"

# Force hot-reload trigger
touch rust.gdextension

echo "Build complete. Artifact safe-swapped to bin/${DLL_NAME}${OS_EXT}"
