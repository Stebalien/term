// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ncurses-compatible database discovery
//!
//! Does not support hashed database, only filesystem!

use std::fs::PathExt;
use std::env;
use std::path::PathBuf;

/// Return path to database entry for `term`
pub fn get_dbpath_for_term(term: &str) -> Option<PathBuf> {
    if term.len() == 0 {
        return None;
    }

    let mut dirs_to_search = Vec::new();
    let first_char = term.char_at(0);

    // Find search directory
    match env::var_os("TERMINFO") {
        Some(dir) => dirs_to_search.push(PathBuf::new(&dir)),
        None => {
            if let Some(mut homedir) = env::home_dir() {
                // ncurses compatibility;
                homedir.push(".terminfo");
                dirs_to_search.push(homedir)
            }
            match env::var("TERMINFO_DIRS") {
                Ok(dirs) => for i in dirs.split(':') {
                    if i == "" {
                        dirs_to_search.push(PathBuf::new("/usr/share/terminfo"));
                    } else {
                        dirs_to_search.push(PathBuf::new(i));
                    }
                },
                // Found nothing in TERMINFO_DIRS, use the default paths:
                // According to  /etc/terminfo/README, after looking at
                // ~/.terminfo, ncurses will search /etc/terminfo, then
                // /lib/terminfo, and eventually /usr/share/terminfo.
                Err(..) => {
                    dirs_to_search.push(PathBuf::new("/etc/terminfo"));
                    dirs_to_search.push(PathBuf::new("/lib/terminfo"));
                    dirs_to_search.push(PathBuf::new("/usr/share/terminfo"));
                }
            }
        }
    };

    // Look for the terminal in all of the search directories
    for mut p in dirs_to_search {
        if p.exists() {
            p.push(&first_char.to_string());
            p.push(&term);
            if p.exists() {
                return Some(p);
            }
            p.pop();
            p.pop();

            // on some installations the dir is named after the hex of the char (e.g. OS X)
            p.push(&format!("{:x}", first_char as usize));
            p.push(term);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}
