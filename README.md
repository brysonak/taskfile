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

## Windows
Download the `tsk-setup.exe` binary from the [releases section](https://github.com/brysonak/taskfile/releases) and run it