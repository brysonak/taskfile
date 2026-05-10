# Taskfile Syntax Reference

A taskfile is a plain text file named `Taskfile.tsk`, `taskfile.tsk`, or `.tsk` placed in your project directory. `tsk` will search the current directory and all parent directories for one.

## Variables

Variables are declared at the top level, outside any task. They are expanded in declaration order, so a variable can reference one declared above it.

```
CC     = gcc
CFLAGS = -Wall -Wextra -O2
FLAGS  = $CC $CFLAGS
```

Reference a variable with `$NAME` or `${NAME}`. Both forms are equivalent; braces are useful when the variable name is adjacent to other text.

```
OUT = build/myapp
BIN = $OUT/main
```

Variables declared later in the file that reference an earlier one capture the value at declaration time, not lazily. Redefining a variable produces a warning but is not an error; the new value takes effect from that point on.

## Tasks

A task is a named block containing directives and shell commands.

```
build {
    gcc main.c -o main
}
```

Run it with:

```
tsk build
```

Everything inside the braces that isn't a recognized directive is passed to the shell as-is. The shell used is `$SHELL` on Unix and `cmd.exe` on Windows. Each command runs as a separate shell invocation, so environment state (like `cd`) is managed by tsk's built-in handlers (see [Built-in Commands](#built-in-commands)).

The opening brace can be on the same line as the task name or on the next line:

```
build {
    gcc main.c -o main
}

# same thing
build
{
    gcc main.c -o main
}
```

## Directives

Directives are lines beginning with `@` inside a task body. They configure the task or affect control flow.

### @default

Marks this task as the one that runs when `tsk` is invoked with no task name.

```
build {
    @default
    gcc main.c -o main
}
```

Only one task should be marked `@default`. If multiple are, the last one wins.

### @desc

Sets a short description shown in `tsk --list`.

```
build {
    @desc Compile the project
    gcc main.c -o main
}
```

### @deps

Declares tasks that must run before this one. Dependencies are run in the order listed and each runs at most once per `tsk` invocation even if multiple tasks depend on it. Circular dependencies are detected and reported as an error.

```
test {
    @deps build
    ./run_tests
}

package {
    @deps build test
    tar -czf dist.tar.gz build/
}
```

### @silent

Suppresses the `$ command` echo that `tsk` prints before each command. The commands still run and their own output is still shown.

```
greet {
    @silent
    echo "Hello"
}
```

### @ignore

Causes command failures (non-zero exit codes) to be treated as warnings rather than errors. Execution continues to the next command in the task. `@error` directives are not affected by `@ignore`.

```
clean {
    @ignore
    rm -f build/main
    rm -f build/
}
```

### @error

Immediately aborts execution and prints a message to stderr. Useful inside conditionals to reject unsupported platforms or configurations.

```
check-platform {
    @deps build
    if $$OS == windows {
        @error("This target does not support Windows")
    }
    ./run_linux_only
}
```

The message can be a plain string or quoted with single or double quotes. Variable expansion is performed on the message before it is printed.

```
REQUIRED_OS = linux

validate {
    if $$OS != $REQUIRED_OS {
        @error("Must be run on $REQUIRED_OS, got $$OS")
    }
}
```

`@error` always exits with code 1, regardless of `@ignore`.

## Variables in Commands

Shell commands inside tasks can reference both user variables and system variables.

User variables use `$NAME` or `${NAME}`:

```
CC = clang

build {
    $CC -o main main.c
    echo "Built with ${CC}"
}
```

System variables use `$$NAME` and are provided by tsk at runtime:

| Variable   | Value                                      |
|------------|--------------------------------------------|
| `$$OS`     | `linux`, `macos`, `windows`, `freebsd`, `openbsd` |
| `$$ARCH`   | Target architecture, e.g. `x86_64`, `aarch64` |
| `$$HOME`   | Home directory                             |
| `$$CWD`    | Current working directory (updated live after `cd`) |
| `$$SHELL`  | Shell binary in use                        |
| `$$USER`   | Current user name                          |

`$$CWD` is always read at the moment the command runs, so it reflects any `cd` calls earlier in the same task.

If a variable is not defined, a warning is printed and the reference expands to an empty string.

## Conditionals

Tasks can branch on conditions using `if`, `else if`, and `else`. Conditions can compare values with `==` or `!=`, or test whether a value is non-empty and not `0` or `false`.

```
deploy {
    @deps build
    if $$OS == linux {
        cp build/main /usr/local/bin/main
    } else if $$OS == macos {
        cp build/main /usr/local/bin/main
        xattr -d com.apple.quarantine /usr/local/bin/main
    } else {
        @error("Unsupported OS: $$OS")
    }
}
```

Both brace styles work for conditionals:

```
# brace on same line
if $RELEASE == true {
    cargo build --release
} else {
    cargo build
}

# brace on next line
if $RELEASE == true
{
    cargo build --release
}
```

Condition operands are variable-expanded before comparison, so you can compare user variables, system variables, or literal strings in any combination.

## Comments

Lines beginning with `#` are ignored. Inline comments after a command are also supported.

```
# This is a full-line comment

CC = gcc  # inline comment on a variable
```

## Built-in Commands

Two commands are handled directly by tsk rather than passed to the shell. This is necessary because each command otherwise runs in its own shell subprocess, so state like the working directory would not carry over between lines.

`cd <dir>` - Changes the working directory for all subsequent commands in the task. With no argument, changes to `$HOME`.

`export KEY=VAL` - Sets an environment variable that will be inherited by subsequent shell commands in the task. On windows, `set KEY=VAL` is also accepted and does the same thing.

## Exit Codes

| Code | Meaning                        |
|------|--------------------------------|
| 0    | Success                        |
| 1    | CLI error or `@error` directive |
| 2    | Syntax error in the taskfile   |
| 3    | Runtime command failure        |

## CLI Reference

```
tsk [FLAGS] [task]

FLAGS:
    -f, --file <path>    Use a specific taskfile instead of searching
    -l, --list           List all tasks and their descriptions
    -n, --dry-run        Print commands without running them
    -s, --silent         Suppress command echo for all tasks
    -h, --help           Print help
    -V, --version        Print version
```

If no task is given, the task marked `@default` runs. If there is no default, the task list is printed.

`tsk` searches for a taskfile starting in the current directory and walking up to the filesystem root, so you can run `tsk` from anywhere inside your project tree.

## Full Example

```bash
CC     = gcc
CFLAGS = -Wall 
BIN    = main

build {
    @default
    @desc Compile the project
    $CC $CFLAGS main.c -o $BIN
}

test {
    @desc Build and run tests
    @deps build
    if $$OS == windows {
        @error("Tests are not supported on Windows")
    }
    ./test_runner
}

install {
    @desc Install to /usr/local/bin
    @deps build
    cp $BIN /usr/local/bin/$BIN
    echo "Installed $BIN"
}

clean {
    @desc Remove build artifacts
    @ignore
    rm -f $BIN
}
```
