use std::collections::HashMap;
use std::process::Command;

use crate::parser::{Condition, Statement, Task, Taskfile};
use crate::error::TskError;
use crate::platform;

pub struct Executor {
    taskfile: Taskfile,
    /// User variables, expanded in definition order so later vars can reference earlier ones
    user_vars: HashMap<String, String>,
    /// System variables ($$-prefixed)
    sys_vars: HashMap<String, String>,
    /// Print commands before running them
    pub echo: bool,
    /// Don't actually run commands
    pub dry_run: bool,
    /// Track tasks currently executing to prevent cycles
    running: Vec<String>,
}

impl Executor {
    pub fn new(taskfile: Taskfile) -> Self {
        // Expand globals in declaration order so later vars can reference earlier ones.
        // e.g.  BASE = -Wall
        //       CFLAGS = $BASE -O2  -> "-Wall -O2"
        let sys_vars = platform::system_vars();
        let mut user_vars: HashMap<String, String> = HashMap::new();

        for name in &taskfile.global_order {
            if let Some((raw, _)) = taskfile.globals.get(name) {
                let expanded = expand_vars(raw, &user_vars, &sys_vars);
                user_vars.insert(name.clone(), expanded);
            }
        }

        Executor {
            taskfile,
            user_vars,
            sys_vars,
            echo: true,
            dry_run: false,
            running: Vec::new(),
        }
    }

    pub fn run(&mut self, task_name: &str) -> Result<(), TskError> {
        if self.running.contains(&task_name.to_string()) {
            return Err(TskError::runtime(
                task_name, 0,
                format!("circular dependency detected: {} -> {}", self.running.join(" -> "), task_name),
                None,
            ));
        }

        let task = match self.taskfile.tasks.get(task_name) {
            Some(t) => t.clone(),
            None => {
                let mut available: Vec<&String> = self.taskfile.tasks.keys().collect();
                available.sort();
                let list = if available.is_empty() {
                    "(none)".to_string()
                } else {
                    available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                };
                return Err(TskError::cli(format!(
                    "task '{}' not found.\nAvailable tasks: {}", task_name, list
                )));
            }
        };

        // Run dependencies first
        let deps = task.deps.clone();
        self.running.push(task_name.to_string());
        for dep in &deps {
            self.run(dep)?;
        }

        self.exec_task(&task)?;
        self.running.pop();
        Ok(())
    }


    fn exec_task(&mut self, task: &Task) -> Result<(), TskError> {
        // Collect the entire task body into a single shell script so that
        // `cd`, `export`, and any other state-carrying commands work
        let mut script_lines: Vec<(String, usize, bool)> = Vec::new(); // (line, src_line, ignore)
        self.collect_statements(&task.body, &mut script_lines)?;

        let silent = task.flags.silent;
        let task_ignore = task.flags.ignore;
        let task_name = task.name.clone();

        for (line, src_line, stmt_ignore) in script_lines {
            let ignore = task_ignore || stmt_ignore;
            self.exec_line(&line, &task_name, src_line, silent, ignore)?;
        }

        Ok(())
    }

    /// Walk statements and collect raw (unexpanded) command lines
    /// Expansion happens at exec time so that dynamic vars like $$CWD reflect
    /// state changes from earlier commands (e.g. cd).
    fn collect_statements(
        &self,
        stmts: &[Statement],
        out: &mut Vec<(String, usize, bool)>,
    ) -> Result<(), TskError> {
        for stmt in stmts {
            match stmt {
                Statement::Command { raw, line } => {
                    // Store raw, expand later at exec_line time
                    out.push((raw.clone(), *line, false));
                }
                Statement::If { condition, then_body, else_ifs, else_body, .. } => {
                    if self.eval_condition(condition) {
                        self.collect_statements(then_body, out)?;
                    } else {
                        let mut matched = false;
                        for ei in else_ifs {
                            if self.eval_condition(&ei.condition) {
                                self.collect_statements(&ei.body, out)?;
                                matched = true;
                                break;
                            }
                        }
                        if !matched {
                            self.collect_statements(else_body, out)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn exec_line(
        &self,
        raw_cmd: &str,
        task_name: &str,
        line: usize,
        silent: bool,
        ignore: bool,
    ) -> Result<(), TskError> {
        let cmd_owned = self.expand(raw_cmd);
        let cmd = cmd_owned.trim();
        if cmd.is_empty() { return Ok(()); }

        if !silent && !self.dry_run {
            eprintln!("  \x1b[2m$ {}\x1b[0m", cmd);
        } else if self.dry_run {
            println!("  $ {}", cmd);
            return Ok(());
        }

        // Built-ins
        if let Some(result) = try_builtin(cmd) {
            return match result {
                Ok(()) => Ok(()),
                Err(e) if ignore => {
                    crate::error::warn(format!("ignored error in '{}': {}", task_name, e));
                    Ok(())
                }
                Err(e) => Err(TskError::runtime(task_name, line, e, Some(cmd.to_string()))),
            };
        }

        let (shell_bin, shell_flag) = platform::shell();
        let status = Command::new(&shell_bin)
            .arg(&shell_flag)
            .arg(cmd)
            .status()
            .map_err(|e| TskError::runtime(
                task_name, line,
                format!("failed to spawn '{}': {}", shell_bin, e),
                Some(cmd.to_string()),
            ))?;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            if ignore {
                crate::error::warn(format!(
                    "task '{}' line {}: command exited {} (ignored)", task_name, line, code
                ));
                return Ok(());
            }
            return Err(TskError::runtime(
                task_name, line,
                format!("exited with status {}", code),
                Some(cmd.to_string()),
            ));
        }

        Ok(())
    }

    pub fn expand(&self, input: &str) -> String {
        expand_vars(input, &self.user_vars, &self.sys_vars)
    }

    fn eval_condition(&self, cond: &Condition) -> bool {
        match cond {
            Condition::Eq(l, r)    => self.expand(l) == self.expand(r),
            Condition::NotEq(l, r) => self.expand(l) != self.expand(r),
            Condition::Truthy(v)   => {
                let s = self.expand(v);
                !s.is_empty() && s != "0" && s != "false"
            }
        }
    }
}

// NOTE: because we run the entire task as separate shell invocations,
// `cd` and `export` must remain built-ins that mutate the tsk process. 
// For full shell-state sharing, wrap the whole task in a shell heredoc or use a shell script
fn try_builtin(cmd: &str) -> Option<Result<(), String>> {
    let (verb, rest) = match cmd.find(char::is_whitespace) {
        Some(i) => (&cmd[..i], cmd[i..].trim()),
        None    => (cmd, ""),
    };
    match verb {
        "cd" => {
            let dir = if rest.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| ".".into())
            } else {
                rest.to_string()
            };
            Some(std::env::set_current_dir(&dir)
                .map_err(|e| format!("cd: {}: {}", dir, e)))
        }
        "export" => {
            if let Some(eq) = rest.find('=') {
                let key = rest[..eq].trim();
                let val = rest[eq+1..].trim();
                unsafe {std::env::set_var(key, val);}
            }
            Some(Ok(()))
        }
        _ => None,
    }
}

pub fn expand_vars(
    input: &str,
    user_vars: &HashMap<String, String>,
    sys_vars: &HashMap<String, String>,
) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '$' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            out.push('$');
            i += 1;
            continue;
        }

        if chars[i + 1] == '$' {
            i += 2;
            let (name, consumed) = read_var_name(&chars[i..]);
            i += consumed;
            if name.is_empty() {
                out.push_str("$$");
            } else if name == "CWD" {
                // Always read CWD dynamically so cd builtin is reflected
                let cwd = std::env::current_dir()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                out.push_str(&cwd);
            } else if let Some(v) = sys_vars.get(&name) {
                out.push_str(v);
            } else if let Ok(v) = std::env::var(&name) {
                // Fall through to real environment
                out.push_str(&v);
            } else {
                crate::error::warn(format!("unknown system variable '$${}'" , name));
            }
        } else if chars[i + 1] == '{' {
            i += 2;
            let end = chars[i..].iter().position(|&c| c == '}');
            match end {
                Some(e) => {
                    let name: String = chars[i..i+e].iter().collect();
                    i += e + 1;
                    if let Some(v) = user_vars.get(&name) {
                        out.push_str(v);
                    } else if let Ok(v) = std::env::var(&name) {
                        out.push_str(&v);
                    } else {
                        crate::error::warn(format!("undefined variable '${{{}}}'", name));
                    }
                }
                None => { out.push_str("${"); }
            }
        } else {
            i += 1;
            let (name, consumed) = read_var_name(&chars[i..]);
            i += consumed;
            if name.is_empty() {
                out.push('$');
            } else if let Some(v) = user_vars.get(&name) {
                out.push_str(v);
            } else if let Ok(v) = std::env::var(&name) {
                out.push_str(&v);
            } else {
                crate::error::warn(format!("undefined variable '${}'", name));
            }
        }
    }

    out
}

fn read_var_name(chars: &[char]) -> (String, usize) {
    let mut name = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_alphanumeric() || c == '_' {
            name.push(c);
        } else {
            return (name, i);
        }
    }
    (name, chars.len())
}
