// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Terminfo database interface.

use std::collections::HashMap;
use std::error::Error as ErrorTrait;
use std::old_io::{IoError, IoErrorKind, IoResult, File};
use std::old_io as io;
use std::os;

use Attr;
use color;
use Terminal;
use UnwrappableTerminal;
use self::searcher::get_dbpath_for_term;
use self::parser::compiled::{parse, msys_terminfo};
use self::parm::{expand, Variables, Param};


/// A parsed terminfo database entry.
#[derive(Show)]
pub struct TermInfo {
    /// Names for the terminal
    pub names: Vec<String> ,
    /// Map of capability name to boolean value
    pub bools: HashMap<String, bool>,
    /// Map of capability name to numeric value
    pub numbers: HashMap<String, u16>,
    /// Map of capability name to raw (unexpanded) string
    pub strings: HashMap<String, Vec<u8> >
}

/// A terminfo creation error.
pub enum Error {
    /// TermUnset Indicates that the environment doesn't include enough information to find
    /// the terminfo entry.
    TermUnset,
    /// MalformedTerminfo indicates that parsing the terminfo entry failed.
    MalformedTerminfo(String),
    /// IoError forwards any IoErrors encountered when finding or reading the terminfo entry.
    IoError(IoError),
}

impl ErrorTrait for Error {
    fn description(&self) -> &str { "failed to create TermInfo" }

    fn detail(&self) -> Option<String> {
        use self::Error::*;
        match self {
            &TermUnset => None,
            &MalformedTerminfo(ref e) => Some(e.clone()),
            &IoError(ref e) => e.detail()
        }
    }

    fn cause(&self) -> Option<&ErrorTrait> {
        use self::Error::*;
        match self {
            &IoError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl TermInfo {
    /// Create a TermInfo based on current environment.
    pub fn from_env() -> Result<TermInfo, Error> {
        let term = match os::getenv("TERM") {
            Some(name) => TermInfo::from_name(&name[]),
            None => return Err(Error::TermUnset),
        };

        if term.is_err() && os::getenv("MSYSCON").map_or(false, |s| "mintty.exe" == s) {
            // msys terminal
            Ok(msys_terminfo())
        } else {
            term
        }
    }

    /// Create a TermInfo for the named terminal.
    pub fn from_name(name: &str) -> Result<TermInfo, Error> {
        get_dbpath_for_term(name).ok_or_else(|| {
            Error::IoError(io::standard_error(IoErrorKind::FileNotFound))
        }).and_then(|p| {
            TermInfo::from_path(&p)
        })
    }

    /// Parse the given TermInfo.
    pub fn from_path(path: &Path) -> Result<TermInfo, Error> {
        File::open(path).map_err(|e| {
            Error::IoError(e)
        }).and_then(|ref mut file| {
            parse(file, false).map_err(|e| {
                Error::MalformedTerminfo(e)
            })
        })
    }
}

pub mod searcher;

/// TermInfo format parsing.
pub mod parser {
    //! ncurses-compatible compiled terminfo format parsing (term(5))
    pub mod compiled;
}
pub mod parm;


fn cap_for_attr(attr: Attr) -> &'static str {
    match attr {
        Attr::Bold               => "bold",
        Attr::Dim                => "dim",
        Attr::Italic(true)       => "sitm",
        Attr::Italic(false)      => "ritm",
        Attr::Underline(true)    => "smul",
        Attr::Underline(false)   => "rmul",
        Attr::Blink              => "blink",
        Attr::Standout(true)     => "smso",
        Attr::Standout(false)    => "rmso",
        Attr::Reverse            => "rev",
        Attr::Secure             => "invis",
        Attr::ForegroundColor(_) => "setaf",
        Attr::BackgroundColor(_) => "setab"
    }
}

/// A Terminal that knows how many colors it supports, with a reference to its
/// parsed Terminfo database record.
pub struct TerminfoTerminal<T> {
    num_colors: u16,
    out: T,
    ti: TermInfo,
}

impl<T: Writer+Send> Terminal<T> for TerminfoTerminal<T> {
    fn fg(&mut self, color: color::Color) -> IoResult<bool> {
        let color = self.dim_if_necessary(color);
        if self.num_colors > color {
            return self.apply_cap("setaf", &[Param::Number(color as int)]);
        }
        Ok(false)
    }

    fn bg(&mut self, color: color::Color) -> IoResult<bool> {
        let color = self.dim_if_necessary(color);
        if self.num_colors > color {
            return self.apply_cap("setab", &[Param::Number(color as int)]);
        }
        Ok(false)
    }

    fn attr(&mut self, attr: Attr) -> IoResult<bool> {
        match attr {
            Attr::ForegroundColor(c) => self.fg(c),
            Attr::BackgroundColor(c) => self.bg(c),
            _ => self.apply_cap(cap_for_attr(attr), &[]),
        }
    }

    fn supports_attr(&self, attr: Attr) -> bool {
        match attr {
            Attr::ForegroundColor(_) | Attr::BackgroundColor(_) => {
                self.num_colors > 0
            }
            _ => {
                let cap = cap_for_attr(attr);
                self.ti.strings.get(cap).is_some()
            }
        }
    }

    fn reset(&mut self) -> IoResult<bool> {
        // are there any terminals that have color/attrs and not sgr0?
        // Try falling back to sgr, then op
        let cmd = match [
            "sg0", "sgr", "op"
        ].iter().filter_map(|cap| {
            self.ti.strings.get(*cap)
        }).next() {
            Some(op) => match expand(&op[], &[], &mut Variables::new()) {
                Ok(cmd) => cmd,
                Err(_) => return Ok(false),
            },
            None => return Ok(false),
        };

        self.out.write(&cmd[]).map(|_|true)
    }

    fn get_ref<'a>(&'a self) -> &'a T { &self.out }

    fn get_mut<'a>(&'a mut self) -> &'a mut T { &mut self.out }
}

impl<T: Writer+Send> UnwrappableTerminal<T> for TerminfoTerminal<T> {
    fn unwrap(self) -> T { self.out }
}

impl<T: Writer+Send> TerminfoTerminal<T> {
    /// Create a new TerminfoTerminal with the given TermInfo and Writer.
    pub fn new_with_terminfo(out: T, terminfo: TermInfo) -> TerminfoTerminal<T> {
        let nc = if terminfo.strings.contains_key("setaf")
                 && terminfo.strings.contains_key("setab") {
                     terminfo.numbers.get("colors").map_or(0, |&n| n)
                 } else { 0 };

        TerminfoTerminal {
            out: out,
            ti: terminfo,
            num_colors: nc,
        }
    }

    /// Create a new TerminfoTerminal for the current environment with the given Writer.
    ///
    /// Returns `None` when the terminfo cannot be found or parsed.
    pub fn new(out: T) -> Option<TerminfoTerminal<T>> {
        TermInfo::from_env().map(move |ti| TerminfoTerminal::new_with_terminfo(out, ti)).ok()
    }

    fn dim_if_necessary(&self, color: color::Color) -> color::Color {
        if color >= self.num_colors && color >= 8 && color < 16 {
            color-8
        } else { color }
    }

    fn apply_cap(&mut self, cmd: &str, params: &[Param]) -> IoResult<bool> {
        if let Some(cmd) = self.ti.strings.get(cmd) {
            if let Ok(s) = expand(cmd.as_slice(), params, &mut Variables::new()) {
                try!(self.out.write(s.as_slice()));
                return Ok(true)
            }
        }
        Ok(false)
    }
}


impl<T: Writer> Writer for TerminfoTerminal<T> {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.out.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.out.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.out.flush()
    }
}
