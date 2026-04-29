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
