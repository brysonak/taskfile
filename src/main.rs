/*
    taskfile - Just Another Task Runner
    Copyright (C) 2026 Bryson Kelly

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

    If you have any questions and or concerns, please contact me @ brysonak@protonmail (dot com)
 */

mod error;
mod executor;
mod lexer;
mod parser;
mod platform;

use error::TskError;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match run(args) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(e.exit_code());
        }
    }
}

fn run(args: Vec<String>) -> Result<(), TskError> {
    let mut taskfile_path: Option<String> = None;
    let mut task_name: Option<String> = None;
    let mut show_list = false;
    let mut dry_run = false;
    let mut no_echo = false;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--file" | "-f" => {
                i += 1;
                if i < args.len() {
                    taskfile_path = Some(args[i].clone());
                } else {
                    return Err(TskError::cli("--file requires a path argument"));
                }
            }
            "--list" | "-l" => show_list = true,
            "--dry-run" | "-n" => dry_run = true,
            "--silent" | "-s" => no_echo = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("tsk {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            arg if !arg.starts_with('-') && task_name.is_none() => {
                task_name = Some(arg.to_string());
            }
            flag => {
                return Err(TskError::cli(format!("unknown flag: {}", flag)));
            }
        }
        i += 1;
    }

    let path = match taskfile_path {
        Some(p) => p,
        None => find_taskfile()?,
    };

    let source = std::fs::read_to_string(&path)
        .map_err(|e| TskError::cli(format!("cannot read '{}': {}", path, e)))?;

    let taskfile = parser::parse(&source, &path)?;

    if show_list {
        print_task_list(&taskfile, &path);
        return Ok(());
    }

    // Resolve task name: explicit > @default > error
    let task = match task_name {
        Some(t) => t,
        None => match &taskfile.default_task {
            Some(d) => d.clone(),
            None => {
                print_task_list(&taskfile, &path);
                return Ok(());
            }
        },
    };

    let mut exec = executor::Executor::new(taskfile);
    exec.echo = !no_echo && !dry_run;
    exec.dry_run = dry_run;
    exec.run(&task)?;

    Ok(())
}

fn print_task_list(taskfile: &parser::Taskfile, path: &str) {
    if taskfile.tasks.is_empty() {
        eprintln!("tsk: no tasks defined in '{}'", path);
        return;
    }
    println!("Available tasks in '{}':", path);
    let mut names: Vec<&String> = taskfile.tasks.keys().collect();
    names.sort();
    for name in names {
        let task = &taskfile.tasks[name];
        let default_marker = if taskfile.default_task.as_deref() == Some(name) {
            " (default)"
        } else {
            ""
        };
        match &task.description {
            Some(d) => println!("  {:<20} {}{}", name, d, default_marker),
            None => println!("  {}{}", name, default_marker),
        }
    }
}

fn find_taskfile() -> Result<String, TskError> {
    let candidates = ["Taskfile.tsk", "taskfile.tsk", ".tsk"];
    let mut dir = env::current_dir().unwrap_or_default();
    loop {
        for c in &candidates {
            let p = dir.join(c);
            if p.exists() {
                return Ok(p.to_string_lossy().into_owned());
            }
        }
        if !dir.pop() {
            break;
        }
    }
    Err(TskError::cli(
        "no Taskfile.tsk found in current or parent directories. Use --file to specify one.",
    ))
}

fn print_help() {
    println!(
        r#"tsk {} - lightweight task runner

USAGE:
    tsk [FLAGS] [task]

FLAGS:
    -f, --file <path>    Use a specific taskfile (default: Taskfile.tsk)
    -l, --list           List all available tasks
    -n, --dry-run        Print commands without running them
    -s, --silent         Suppress command echo
    -h, --help           Print this help
    -V, --version        Print version
"#,
        env!("CARGO_PKG_VERSION")
    );
}
