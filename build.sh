#!/bin/bash
# Stop execution on error.
set -e

echo "Installing Rust"

# Download "rustup-init" script from the official site and execute it with the "-y" option.
# "-y" option disables the confirmation prompt, which effectively downloads Rust and 
# cargo (the package manager) with the default (newest and stable) settings.
# The official rust installation guide: https://rust-lang.org/tools/install/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Load current settings for Rust to work in the current terminal session. 
. "$HOME/.cargo/env"

echo "Rust installed"

echo "Compiling Latte Compiler"
make
