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

use std::old_io::fs::PathExtensions;
use std::env;
use std::os;

/// Return path to database entry for `term`
pub fn get_dbpath_for_term(term: &str) -> Option<Path> {
    if term.len() == 0 {
        return None;
    }

    let homedir = os::homedir();

    let mut dirs_to_search = Vec::new();
    let first_char = term.char_at(0);

    // Find search directory
    match env::var("TERMINFO") {
        Ok(dir) => dirs_to_search.push(Path::new(dir)),
        Err(..) => {
            if homedir.is_some() {
                // ncurses compatibility;
                dirs_to_search.push(homedir.unwrap().join(".terminfo"))
            }
            match env::var("TERMINFO_DIRS") {
                Ok(dirs) => for i in dirs.split(':') {
                    if i == "" {
                        dirs_to_search.push(Path::new("/usr/share/terminfo"));
                    } else {
                        dirs_to_search.push(Path::new(i));
                    }
                },
                // Found nothing in TERMINFO_DIRS, use the default paths:
                // According to  /etc/terminfo/README, after looking at
                // ~/.terminfo, ncurses will search /etc/terminfo, then
                // /lib/terminfo, and eventually /usr/share/terminfo.
                Err(..) => {
                    dirs_to_search.push(Path::new("/etc/terminfo"));
                    dirs_to_search.push(Path::new("/lib/terminfo"));
                    dirs_to_search.push(Path::new("/usr/share/terminfo"));
                }
            }
        }
    };

    // Look for the terminal in all of the search directories
    for p in dirs_to_search.iter() {
        if p.exists() {
            let f: &str = &first_char.to_string();
            let newp = p.join_many(&[f, term]);
            if newp.exists() {
                return Some(newp);
            }
            // on some installations the dir is named after the hex of the char (e.g. OS X)
            let f: &str = &format!("{:x}", first_char as usize);
            let newp = p.join_many(&[f, term]);
            if newp.exists() {
                return Some(newp);
            }
        }
    }
    None
}
