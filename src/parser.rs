use crate::error::{TskError, warn};
use crate::lexer::{Token, TokenKind, lex};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Condition {
    Eq(String, String),
    NotEq(String, String),
    Truthy(String),
}

#[derive(Debug, Clone)]
pub struct ElseIf {
    pub condition: Condition,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    /// A shell command line.
    Command { raw: String, line: usize },
    /// if / else if* / else
    If {
        condition: Condition,
        then_body: Vec<Statement>,
        else_ifs: Vec<ElseIf>,
        else_body: Vec<Statement>,
        line: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TaskFlags {
    pub silent: bool, // @silent — don't echo commands
    pub ignore: bool, // @ignore — continue on command failure
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    pub deps: Vec<String>,
    pub flags: TaskFlags,
    pub body: Vec<Statement>,
    pub is_default: bool,
}

pub struct Taskfile {
    pub source_path: String,
    pub globals: HashMap<String, (String, usize)>,
    pub global_order: Vec<String>,
    pub tasks: HashMap<String, Task>,
    pub default_task: Option<String>,
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    file: &'a str,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], file: &'a str) -> Self {
        Parser {
            tokens,
            pos: 0,
            file,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_newline(&mut self) -> Result<(), TskError> {
        match self.peek().kind {
            TokenKind::Newline | TokenKind::Eof => {
                self.advance();
                Ok(())
            }
            _ => {
                let t = self.peek().clone();
                Err(TskError::syntax(self.file, t.line, "expected newline"))
            }
        }
    }

    fn expect_lbrace(&mut self) -> Result<usize, TskError> {
        self.skip_newlines();
        let t = self.peek().clone();
        match t.kind {
            TokenKind::LBrace => {
                self.advance();
                self.skip_newlines();
                Ok(t.line)
            }
            _ => Err(TskError::syntax(self.file, t.line, "expected '{'")),
        }
    }

    fn expect_rbrace(&mut self) -> Result<(), TskError> {
        self.skip_newlines();
        let t = self.peek().clone();
        match t.kind {
            TokenKind::RBrace => {
                self.advance();
                self.skip_newlines();
                Ok(())
            }
            _ => Err(TskError::syntax(self.file, t.line, "expected '}'")),
        }
    }

    fn parse_taskfile(&mut self) -> Result<Taskfile, TskError> {
        let mut globals: HashMap<String, (String, usize)> = HashMap::new();
        let mut global_order: Vec<String> = Vec::new();
        let mut tasks: HashMap<String, Task> = HashMap::new();
        let mut default_task: Option<String> = None;

        loop {
            self.skip_newlines();
            match self.peek().kind.clone() {
                TokenKind::Eof => break,

                TokenKind::Ident(ref name) => {
                    let name = name.clone();
                    let var_line = self.peek().line;

                    // if next meaningful token after ident is `=`, it's a var.
                    // If it's `{`, it's a task.
                    let next_pos = self.pos + 1;
                    let is_assignment = next_pos < self.tokens.len()
                        && matches!(self.tokens[next_pos].kind, TokenKind::Equals);

                    if is_assignment {
                        self.advance(); // consume ident
                        self.advance(); // consume `=`
                        let val_tok = self.peek().clone();
                        let value = match val_tok.kind {
                            TokenKind::RawValue(ref v) => {
                                self.advance();
                                v.clone()
                            }
                            TokenKind::Newline | TokenKind::Eof => String::new(),
                            _ => {
                                return Err(TskError::syntax(
                                    self.file,
                                    val_tok.line,
                                    "expected value after '='",
                                ));
                            }
                        };
                        self.expect_newline()?;
                        if globals.contains_key(&name) {
                            warn(format!(
                                "variable '{}' redefined at line {}",
                                name, var_line
                            ));
                        } else {
                            global_order.push(name.clone());
                        }
                        globals.insert(name, (value, var_line));
                    } else {
                        // Task definition
                        let task_line = var_line;
                        self.advance(); // consume task name

                        // Check for `@default` marker before the brace
                        self.expect_lbrace()?;
                        let task = self.parse_task_block(&name, task_line)?;
                        self.expect_rbrace()?;

                        if task.flags.silent == false
                            && task.description.as_deref() == Some("@default")
                        {
                        }

                        if tasks.contains_key(&name) {
                            warn(format!("task '{}' redefined at line {}", name, task_line));
                        }

                        // Check if this task declared itself as default
                        if task.is_default {
                            default_task = Some(name.clone());
                        }

                        tasks.insert(name, task);
                    }
                }

                _ => {
                    let t = self.peek().clone();
                    return Err(TskError::syntax(
                        self.file,
                        t.line,
                        format!("unexpected token at top level: {:?}", t.kind),
                    ));
                }
            }
        }

        Ok(Taskfile {
            source_path: self.file.to_string(),
            globals,
            global_order,
            tasks,
            default_task,
        })
    }

    fn parse_task_block(&mut self, name: &str, _task_line: usize) -> Result<Task, TskError> {
        let mut description: Option<String> = None;
        let mut deps: Vec<String> = Vec::new();
        let mut flags = TaskFlags::default();
        let mut body = Vec::new();
        let mut is_default = false;

        loop {
            self.skip_newlines();
            match self.peek().kind.clone() {
                TokenKind::RBrace | TokenKind::Eof => break,

                TokenKind::Ident(ref kw) => {
                    let kw = kw.clone();
                    match kw.as_str() {
                        "@desc" => {
                            self.advance();
                            let val = match self.peek().kind.clone() {
                                TokenKind::RawValue(v) => {
                                    self.advance();
                                    v
                                }
                                _ => String::new(),
                            };
                            description = Some(val);
                            self.skip_newlines();
                        }
                        "@default" => {
                            self.advance();
                            is_default = true;
                            self.skip_newlines();
                        }
                        "@deps" => {
                            self.advance();
                            // Remaining tokens on this line are dep names
                            match self.peek().kind.clone() {
                                TokenKind::RawValue(v) => {
                                    self.advance();
                                    for dep in v.split_whitespace() {
                                        deps.push(dep.to_string());
                                    }
                                }
                                _ => {}
                            }
                            self.skip_newlines();
                        }
                        "@silent" => {
                            self.advance();
                            flags.silent = true;
                            self.skip_newlines();
                        }
                        "@ignore" => {
                            self.advance();
                            flags.ignore = true;
                            self.skip_newlines();
                        }
                        "if" => {
                            let if_line = self.peek().line;
                            self.advance();
                            body.push(self.parse_if(if_line)?);
                        }
                        _ => {
                            let cmd_line = self.peek().line;
                            self.advance();
                            self.skip_newlines();
                            body.push(Statement::Command {
                                raw: kw,
                                line: cmd_line,
                            });
                        }
                    }
                }

                TokenKind::RawValue(cmd) => {
                    let cmd_line = self.peek().line;
                    self.advance();
                    self.skip_newlines();
                    body.push(Statement::Command {
                        raw: cmd,
                        line: cmd_line,
                    });
                }

                _ => {
                    let t = self.peek().clone();
                    return Err(TskError::syntax(
                        self.file,
                        t.line,
                        "unexpected token in task body",
                    ));
                }
            }
        }

        Ok(Task {
            name: name.to_string(),
            description,
            deps,
            flags,
            body,
            is_default,
        })
    }

    fn parse_if(&mut self, if_line: usize) -> Result<Statement, TskError> {
        let condition = self.parse_condition(if_line)?;
        self.expect_lbrace()?;
        let then_body = self.parse_body_until_rbrace()?;
        self.expect_rbrace()?;

        let mut else_ifs: Vec<ElseIf> = Vec::new();
        let mut else_body: Vec<Statement> = Vec::new();

        loop {
            let is_else = match &self.peek().kind {
                TokenKind::Ident(k) if k == "else" => true,
                _ => false,
            };
            if !is_else {
                break;
            }

            let else_line = self.peek().line;
            self.advance(); // consume `else`

            // `else if`?
            let is_else_if = match &self.peek().kind {
                TokenKind::Ident(k) if k == "if" => true,
                _ => false,
            };

            if is_else_if {
                self.advance(); // consume `if`
                let cond = self.parse_condition(else_line)?;
                self.expect_lbrace()?;
                let body = self.parse_body_until_rbrace()?;
                self.expect_rbrace()?;
                else_ifs.push(ElseIf {
                    condition: cond,
                    body,
                });
            } else {
                // plain `else`
                self.expect_lbrace()?;
                else_body = self.parse_body_until_rbrace()?;
                self.expect_rbrace()?;
                break;
            }
        }

        Ok(Statement::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            line: if_line,
        })
    }

    fn parse_condition(&mut self, if_line: usize) -> Result<Condition, TskError> {
        let lhs_tok = self.peek().clone();
        let lhs = match &lhs_tok.kind {
            TokenKind::RawValue(v) => {
                self.advance();
                v.clone()
            }
            TokenKind::LBrace => return Ok(Condition::Truthy("1".to_string())),
            _ => {
                return Err(TskError::syntax(
                    self.file,
                    if_line,
                    "expected condition after 'if'",
                ));
            }
        };

        match self.peek().kind.clone() {
            TokenKind::EqEq => {
                self.advance();
                let rhs = self.expect_raw_value()?;
                Ok(Condition::Eq(lhs, rhs))
            }
            TokenKind::NotEq => {
                self.advance();
                let rhs = self.expect_raw_value()?;
                Ok(Condition::NotEq(lhs, rhs))
            }
            TokenKind::LBrace | TokenKind::Newline => Ok(Condition::Truthy(lhs)),
            _ => Err(TskError::syntax(
                self.file,
                if_line,
                "expected '==', '!=', or '{'",
            )),
        }
    }

    fn expect_raw_value(&mut self) -> Result<String, TskError> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::RawValue(v) => {
                self.advance();
                Ok(v)
            }
            _ => Err(TskError::syntax(self.file, t.line, "expected value")),
        }
    }

    fn parse_body_until_rbrace(&mut self) -> Result<Vec<Statement>, TskError> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().kind.clone() {
                TokenKind::RBrace | TokenKind::Eof => break,
                TokenKind::Ident(ref kw) if kw == "if" => {
                    let line = self.peek().line;
                    self.advance();
                    stmts.push(self.parse_if(line)?);
                }
                TokenKind::RawValue(cmd) => {
                    let line = self.peek().line;
                    self.advance();
                    self.skip_newlines();
                    stmts.push(Statement::Command { raw: cmd, line });
                }
                TokenKind::Ident(word) => {
                    let line = self.peek().line;
                    self.advance();
                    self.skip_newlines();
                    stmts.push(Statement::Command { raw: word, line });
                }
                _ => {
                    let t = self.peek().clone();
                    return Err(TskError::syntax(
                        self.file,
                        t.line,
                        "unexpected token inside block",
                    ));
                }
            }
        }
        Ok(stmts)
    }
}

pub fn parse(source: &str, file: &str) -> Result<Taskfile, TskError> {
    let tokens = lex(source);
    let mut parser = Parser::new(&tokens, file);
    let taskfile = parser.parse_taskfile()?;

    for (name, task) in &taskfile.tasks {
        if task.body.is_empty() && task.deps.is_empty() {
            warn(format!("task '{}' has an empty body", name));
        }
    }

    Ok(taskfile)
}
