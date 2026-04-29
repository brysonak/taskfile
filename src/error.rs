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

use std::fmt;

/// All error kinds tsk can produce, with distinct exit codes
#[derive(Debug)]
pub enum TskError {
    /// Bad CLI invocation (exit 1)
    Cli(String),
    /// Syntax / parse error (exit 2)
    Syntax {
        file: String,
        line: usize,
        message: String,
    },
    /// Runtime error during task execution (exit 3)
    Runtime {
        task: String,
        line: usize,
        message: String,
        command: Option<String>,
    },
}

impl TskError {
    pub fn cli(msg: impl Into<String>) -> Self {
        TskError::Cli(msg.into())
    }

    pub fn syntax(file: impl Into<String>, line: usize, msg: impl Into<String>) -> Self {
        TskError::Syntax {
            file: file.into(),
            line,
            message: msg.into(),
        }
    }

    pub fn runtime(
        task: impl Into<String>,
        line: usize,
        msg: impl Into<String>,
        cmd: Option<String>,
    ) -> Self {
        TskError::Runtime {
            task: task.into(),
            line,
            message: msg.into(),
            command: cmd,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            TskError::Cli(_) => 1,
            TskError::Syntax { .. } => 2,
            TskError::Runtime { .. } => 3,
        }
    }
}

impl fmt::Display for TskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TskError::Cli(msg) => {
                write!(f, "tsk: {}\nRun 'tsk --help' for usage.", msg)
            }
            TskError::Syntax {
                file,
                line,
                message,
            } => {
                write!(f, "tsk: syntax error in {}:{}: {}", file, line, message)
            }
            TskError::Runtime {
                task,
                line,
                message,
                command,
            } => {
                write!(
                    f,
                    "tsk: runtime error in task '{}' at line {}: {}",
                    task, line, message
                )?;
                if let Some(cmd) = command {
                    write!(f, "\n  command: {}", cmd)?;
                }
                Ok(())
            }
        }
    }
}

/// Non-fatal warning, printed to stderr, never stops execution
pub fn warn(msg: impl AsRef<str>) {
    eprintln!("tsk: warning: {}", msg.as_ref());
}
