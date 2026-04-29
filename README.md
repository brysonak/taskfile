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
Then, you'd run ``tsk`` in that directory, and that's it!

I plan to write lots of documentation for this in the time that follows

# Build Instructions
Coming soon *tm*

The tool is a little more oriented towards linux/macos right now, but I am testing a windows build soon too.