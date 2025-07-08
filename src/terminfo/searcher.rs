// Copyright 2019 The Rust Project Developers. See the COPYRIGHT
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

use std::env;
use std::fs;
use std::path::PathBuf;

// The default terminfo location should be /usr/lib/terminfo but that's not guaranteed, so we check
// a few more locations. See https://tldp.org/HOWTO/Text-Terminal-HOWTO-16.html#ss16.2
const DEFAULT_LOCATIONS: &[&str] = &[
    "/etc/terminfo",
    "/usr/share/terminfo",
    "/usr/lib/terminfo",
    "/lib/terminfo",
    #[cfg(target_os = "haiku")]
    "/boot/system/data/terminfo",
];

/// Return path to database entry for `term`
pub fn get_dbpath_for_term(term: &str) -> Option<PathBuf> {
    let mut dirs_to_search = Vec::new();
    let mut default_locations = DEFAULT_LOCATIONS.iter().map(PathBuf::from);
    let first_char = term.chars().next()?;

    // From the manual.
    //
    // > The  environment  variable TERMINFO is checked first, for a terminal
    // > database containing the terminal description.
    if let Some(dir) = env::var_os("TERMINFO") {
        dirs_to_search.push(PathBuf::from(dir));
    }

    // > Next, ncurses looks in $HOME/.terminfo for a compiled description.
    if let Some(mut homedir) = env::home_dir() {
        homedir.push(".terminfo");
        dirs_to_search.push(homedir)
    }

    // > Next, if the environment variable TERMINFO_DIRS is set, ncurses interprets
    // > the contents of that variable as a list of colon-separated pathnames of
    // > terminal databases to be searched.
    // >
    // > An  empty  pathname  (i.e.,  if  the  variable begins or ends with a
    // > colon, or contains adjacent colons) is interpreted as the system location
    // > /usr/share/terminfo.
    if let Ok(dirs) = env::var("TERMINFO_DIRS") {
        for i in dirs.split(':') {
            if i.is_empty() {
                dirs_to_search.extend(&mut default_locations);
            } else {
                dirs_to_search.push(PathBuf::from(i));
            }
        }
    }

    // > Finally, ncurses searches these compiled-in locations...
    //
    // NOTE: We only append these to `dirs_to_search` once. If we've already added these
    // directories as specified in `TERMINFO_DIRS`, this operation will be a no-op.
    dirs_to_search.extend(&mut default_locations);

    // Look for the terminal in all of the search directories
    for mut p in dirs_to_search {
        if fs::metadata(&p).is_ok() {
            p.push(first_char.to_string());
            p.push(term);
            if fs::metadata(&p).is_ok() {
                return Some(p);
            }
            p.pop();
            p.pop();

            // on some installations the dir is named after the hex of the char
            // (e.g. OS X)
            p.push(format!("{:x}", first_char as usize));
            p.push(term);
            if fs::metadata(&p).is_ok() {
                return Some(p);
            }
        }
    }
    None
}
