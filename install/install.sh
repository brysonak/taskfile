#!/bin/sh

REPO="https://github.com/brysonak/taskfile.git"
INSTALL_DIR="/usr/bin"
BIN="tsk"

check_dep() {
    if ! command -v "$1" > /dev/null 2>&1; then
        echo "$1 is needed to run this, please install it before running this script again."
        exit 1
    fi
}

check_dep git
check_dep cargo
check_dep cc

CLONE_DIR=$(mktemp -d)

git clone "$REPO" "$CLONE_DIR"
cd "$CLONE_DIR"
cargo build --release
sudo cp "target/release/$BIN" "$INSTALL_DIR/$BIN"

echo "tsk installed, restart your shell to use it"

cd /
rm -rf "$CLONE_DIR"