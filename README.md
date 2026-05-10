# Taskfile
Taskfile is a tool similar to Just or Make, but easier syntax and written in rust

# What does taskfile look like?
A barebones way to build a simple C file would go like this:
```
CC = gcc

build {
    @default
    $CC main.c -o main
}
```

See [docs/syntax.md](docs/syntax.md) for the full language reference

# Installing

## Linux
Run:
```bash
curl -fsSL https://raw.githubusercontent.com/brysonak/taskfile/refs/heads/main/install/install.sh | sh
```
**Note:** This will ask for permissions, as it copies the binary to `/usr/bin`

## Windows
Download the `tsk-setup.exe` binary from the [releases section](https://github.com/brysonak/taskfile/releases) and run it

# Building
Before you build, make sure you have the following pre-requisites:
- [git](https://git-scm.com/)
- [rust](https://rust-lang.org/tools/install/)

**Run these commands in order**
```bash
git clone https://github.com/brysonak/taskfile.git

cd taskfile

cargo build --release
```