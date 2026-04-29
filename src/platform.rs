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

use std::collections::HashMap;
use std::env;

/// Populate the `$$`prefixed system variables available at runtime
pub fn system_vars() -> HashMap<String, String> {
    let mut map = HashMap::new();

    map.insert("OS".to_string(), detect_os());
    map.insert("ARCH".to_string(), env::consts::ARCH.to_string());

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();
    map.insert("HOME".to_string(), home);

    let cwd = env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    map.insert("CWD".to_string(), cwd);

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    map.insert("SHELL".to_string(), shell);

    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_default();
    map.insert("USER".to_string(), user);

    map
}

fn detect_os() -> String {
    if cfg!(target_os = "linux") {
        return "linux".to_string();
    }
    if cfg!(target_os = "macos") {
        return "macos".to_string();
    }
    if cfg!(target_os = "windows") {
        return "windows".to_string();
    }
    if cfg!(target_os = "freebsd") {
        return "freebsd".to_string();
    }
    if cfg!(target_os = "openbsd") {
        return "openbsd".to_string();
    }
    env::consts::OS.to_string()
}

/// Returns `(shell_binary, flag)` for spawning commands
pub fn shell() -> (String, String) {
    if cfg!(target_os = "windows") {
        ("cmd".to_string(), "/C".to_string())
    } else {
        let sh = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        (sh, "-c".to_string())
    }
}
