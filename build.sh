#!/usr/bin/env zsh

# Exit immediately if any command fails
set -e

cargo build

# Create bin directory if it doesn't exist
mkdir -p bin

# Copy the DLL
cp -f target/debug/texel_splatting.dll bin/texel_splatting.dll

# Only attempt to copy the .pdb if it actually exists
if [[ -f target/debug/texel_splatting.pdb ]]; then
    cp -f target/debug/texel_splatting.pdb bin/texel_splatting.pdb
fi

echo "[+] gdext hot-reload build complete!"
