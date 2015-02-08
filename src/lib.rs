// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Terminal formatting library.
//!
//! This crate provides the `Terminal` trait, which abstracts over an [ANSI
//! Terminal][ansi] to provide color printing, among other things. There are two implementations,
//! the `TerminfoTerminal`, which uses control characters from a
//! [terminfo][ti] database, and `WinConsole`, which uses the [Win32 Console
//! API][win].
//!
//! # Examples
//!
//! ```no_run
//! # #![allow(unstable)]
//! extern crate term;
//!
//! fn main() {
//!     let mut t = term::stdout().unwrap();
//!
//!     t.fg(term::color::GREEN).unwrap();
//!     (write!(t, "hello, ")).unwrap();
//!
//!     t.fg(term::color::RED).unwrap();
//!     (writeln!(t, "world!")).unwrap();
//!
//!     assert!(t.reset().unwrap());
//! }
//! ```
//!
//! [ansi]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [win]: http://msdn.microsoft.com/en-us/library/windows/desktop/ms682010%28v=vs.85%29.aspx
//! [ti]: https://en.wikipedia.org/wiki/Terminfo

#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://doc.rust-lang.org/nightly/",
       html_playground_url = "http://play.rust-lang.org/")]
#![feature(int_uint)]
#![deny(missing_docs)]
#![allow(unstable)]

#[macro_use] extern crate log;

pub use terminfo::TerminfoTerminal;
#[cfg(windows)]
pub use win::WinConsole;

use std::old_io::{IoResult, LineBufferedWriter};
use std::old_io::stdio::StdWriter;
use std::old_io as io;

pub mod terminfo;

#[cfg(windows)]
mod win;

/// Alias for stderr/stdout terminals.
pub type StdTerminal = Terminal<LineBufferedWriter<StdWriter>> + Send;

#[cfg(not(windows))]
/// Return a Terminal wrapping stdout, or None if a terminal couldn't be
/// opened.
pub fn stdout() -> Option<Box<StdTerminal>> {
    TerminfoTerminal::new(io::stdout()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    })
}

#[cfg(windows)]
/// Return a Terminal wrapping stdout, or None if a terminal couldn't be
/// opened.
pub fn stdout() -> Option<Box<StdTerminal>> {
    TerminfoTerminal::new(io::stdout()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    }).or_else(|| WinConsole::new(io::stdout()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    }))
}

#[cfg(not(windows))]
/// Return a Terminal wrapping stderr, or None if a terminal couldn't be
/// opened.
pub fn stderr() -> Option<Box<StdTerminal>> {
    TerminfoTerminal::new(io::stderr()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    })
}

#[cfg(windows)]
/// Return a Terminal wrapping stderr, or None if a terminal couldn't be
/// opened.
pub fn stderr() -> Option<Box<StdTerminal>> {
    TerminfoTerminal::new(io::stderr()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    }).or_else(|| WinConsole::new(io::stderr()).map(|t| {
        Box::new(t) as Box<StdTerminal>
    }))
}


/// Terminal color definitions
pub mod color {
    /// Number for a terminal color
    pub type Color = u16;

    pub const BLACK:   Color = 0;
    pub const RED:     Color = 1;
    pub const GREEN:   Color = 2;
    pub const YELLOW:  Color = 3;
    pub const BLUE:    Color = 4;
    pub const MAGENTA: Color = 5;
    pub const CYAN:    Color = 6;
    pub const WHITE:   Color = 7;

    pub const BRIGHT_BLACK:   Color = 8;
    pub const BRIGHT_RED:     Color = 9;
    pub const BRIGHT_GREEN:   Color = 10;
    pub const BRIGHT_YELLOW:  Color = 11;
    pub const BRIGHT_BLUE:    Color = 12;
    pub const BRIGHT_MAGENTA: Color = 13;
    pub const BRIGHT_CYAN:    Color = 14;
    pub const BRIGHT_WHITE:   Color = 15;
}

/// Terminal attributes for use with term.attr().
///
/// Most attributes can only be turned on and must be turned off with term.reset().
/// The ones that can be turned off explicitly take a boolean value.
/// Color is also represented as an attribute for convenience.
#[derive(Show, PartialEq, Eq, Copy)]
pub enum Attr {
    /// Bold (or possibly bright) mode
    Bold,
    /// Dim mode, also called faint or half-bright. Often not supported
    Dim,
    /// Italics mode. Often not supported
    Italic(bool),
    /// Underline mode
    Underline(bool),
    /// Blink mode
    Blink,
    /// Standout mode. Often implemented as Reverse, sometimes coupled with Bold
    Standout(bool),
    /// Reverse mode, inverts the foreground and background colors
    Reverse,
    /// Secure mode, also called invis mode. Hides the printed text
    Secure,
    /// Convenience attribute to set the foreground color
    ForegroundColor(color::Color),
    /// Convenience attribute to set the background color
    BackgroundColor(color::Color)
}

/// A terminal with similar capabilities to an ANSI Terminal
/// (foreground/background colors etc).
pub trait Terminal<T: Writer>: Writer {
    /// Sets the foreground color to the given color.
    ///
    /// If the color is a bright color, but the terminal only supports 8 colors,
    /// the corresponding normal color will be used instead.
    ///
    /// Returns `Ok(true)` if the color was set, `Ok(false)` otherwise, and `Err(e)`
    /// if there was an I/O error.
    fn fg(&mut self, color: color::Color) -> IoResult<bool>;

    /// Sets the background color to the given color.
    ///
    /// If the color is a bright color, but the terminal only supports 8 colors,
    /// the corresponding normal color will be used instead.
    ///
    /// Returns `Ok(true)` if the color was set, `Ok(false)` otherwise, and `Err(e)`
    /// if there was an I/O error.
    fn bg(&mut self, color: color::Color) -> IoResult<bool>;

    /// Sets the given terminal attribute, if supported.  Returns `Ok(true)`
    /// if the attribute was supported, `Ok(false)` otherwise, and `Err(e)` if
    /// there was an I/O error.
    fn attr(&mut self, attr: Attr) -> IoResult<bool>;

    /// Returns whether the given terminal attribute is supported.
    fn supports_attr(&self, attr: Attr) -> bool;

    /// Resets all terminal attributes and color to the default.
    ///
    /// Returns `Ok(true)` if the terminal was reset, `Ok(false)` otherwise, and `Err(e)` if there
    /// was an I/O error.
    fn reset(&mut self) -> IoResult<bool>;

    /// Gets an immutable reference to the stream inside
    fn get_ref<'a>(&'a self) -> &'a T;

    /// Gets a mutable reference to the stream inside
    fn get_mut<'a>(&'a mut self) -> &'a mut T;
}

/// A terminal which can be unwrapped.
pub trait UnwrappableTerminal<T: Writer>: Terminal<T> {
    /// Returns the contained stream, destroying the `Terminal`
    fn unwrap(self) -> T;
}
