// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Parameterized string expansion

use self::Param::*;
use self::States::*;
use self::FormatState::*;
use self::FormatOp::*;

use std::iter::repeat;

#[derive(Clone, Copy, PartialEq)]
enum States {
    Nothing,
    Percent,
    SetVar,
    GetVar,
    PushParam,
    CharConstant,
    CharClose,
    IntConstant(i32),
    FormatPattern(Flags, FormatState),
    SeekIfElse(usize),
    SeekIfElsePercent(usize),
    SeekIfEnd(usize),
    SeekIfEndPercent(usize),
}

#[derive(Copy, PartialEq, Clone)]
enum FormatState {
    FormatStateFlags,
    FormatStateWidth,
    FormatStatePrecision,
}

/// Types of parameters a capability can use
#[allow(missing_docs)]
#[derive(Clone)]
pub enum Param {
    Words(String),
    Number(i32),
}

/// Container for static and dynamic variable arrays
pub struct Variables {
    /// Static variables A-Z
    sta: [Param; 26],
    /// Dynamic variables a-z
    dyn: [Param; 26],
}

impl Variables {
    /// Return a new zero-initialized Variables
    pub fn new() -> Variables {
        Variables {
            sta: [
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0)
            ],
            dyn: [
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
                Number(0), Number(0), Number(0), Number(0), Number(0)
            ],
        }
    }
}

/// Expand a parameterized capability
///
/// # Arguments
/// * `cap`    - string to expand
/// * `params` - vector of params for %p1 etc
/// * `vars`   - Variables struct for %Pa etc
///
/// To be compatible with ncurses, `vars` should be the same between calls to `expand` for
/// multiple capabilities for the same terminal.
pub fn expand(cap: &[u8], params: &[Param], vars: &mut Variables) -> Result<Vec<u8>, String> {
    let mut state = Nothing;

    // expanded cap will only rarely be larger than the cap itself
    let mut output = Vec::with_capacity(cap.len());

    let mut stack: Vec<Param> = Vec::new();

    // Copy parameters into a local vector for mutability
    let mut mparams = [
    	Number(0), Number(0), Number(0), Number(0), Number(0), Number(0),
        Number(0), Number(0), Number(0)
    ];
    for (dst, src) in mparams.iter_mut().zip(params.iter()) {
        *dst = (*src).clone();
    }

    for &c in cap.iter() {
        let cur = c as char;
        let mut old_state = state;
        match state {
            Nothing => {
                if cur == '%' {
                    state = Percent;
                } else {
                    output.push(c);
                }
            }
            Percent => {
                match cur {
                    '%' => {
                        output.push(c);
                        state = Nothing
                    }
                    'c' => {
                        match stack.pop() {
                            // if c is 0, use 0200 (128) for ncurses compatibility
                            Some(Number(0)) => output.push(128u8),
                            // Don't check bounds. ncurses just casts and truncates.
                            Some(Number(c)) => output.push(c as u8),
                            Some(_) => return Err("a non-char was used with %c".to_string()),
                            None => return Err("stack is empty".to_string()),
                        }
                    }
                    'p' => state = PushParam,
                    'P' => state = SetVar,
                    'g' => state = GetVar,
                    '\'' => state = CharConstant,
                    '{' => state = IntConstant(0),
                    'l' => {
                        match stack.pop() {
                            Some(Words(s)) => stack.push(Number(s.len() as i32)),
                            Some(_) => return Err("a non-str was used with %l".to_string()),
                            None => return Err("stack is empty".to_string()),
                        }
                    }
                    '+' | '-' | '/' | '*' | '^' | '&' | '|' | 'm' => {
                        match (stack.pop(), stack.pop()) {
                            (Some(Number(y)), Some(Number(x))) => {
                                stack.push(Number(match cur {
                                    '+' => x + y,
                                    '-' => x - y,
                                    '*' => x * y,
                                    '/' => x / y,
                                    '|' => x | y,
                                    '&' => x & y,
                                    '^' => x ^ y,
                                    'm' => x % y,
                                    _ => unreachable!("All cases handled"),
                                }))
                            }
                            (Some(_), Some(_)) => {
                                return Err(format!("non-numbers on stack with {}", cur))
                            }
                            _ => return Err("stack is empty".to_string()),
                        }
                    }
                    '=' | '>' | '<' | 'A' | 'O' => {
                        match (stack.pop(), stack.pop()) {
                            (Some(Number(y)), Some(Number(x))) => {
                                stack.push(Number(if match cur {
                                    '=' => x == y,
                                    '<' => x < y,
                                    '>' => x > y,
                                    'A' => x > 0 && y > 0,
                                    'O' => x > 0 || y > 0,
                                    _ => unreachable!(),
                                } {
                                    1
                                } else {
                                    0
                                }))
                            }
                            (Some(_), Some(_)) => {
                                return Err(format!("non-numbers on stack with {}", cur))
                            }
                            _ => return Err("stack is empty".to_string()),
                        }
                    }
                    '!' | '~' => {
                        match stack.pop() {
                            Some(Number(x)) => {
                                stack.push(Number(match cur {
                                    '!' if x > 0 => 0,
                                    '!' => 1,
                                    '~' => !x,
                                    _ => unreachable!(),
                                }))
                            }
                            Some(_) => return Err(format!("non-numbers on stack with {}", cur)),
                            None => return Err("stack is empty".to_string()),
                        }
                    }
                    'i' => {
                        match (&mparams[0], &mparams[1]) {
                            (&Number(x), &Number(y)) => {
                                mparams[0] = Number(x + 1);
                                mparams[1] = Number(y + 1);
                            }
                            (_, _) => {
                                return Err("first two params not numbers with %i".to_string())
                            }
                        }
                    }

                    // printf-style support for %doxXs
                    'd' | 'o' | 'x' | 'X' | 's' => {
                        if let Some(arg) = stack.pop() {
                            let flags = Flags::new();
                            let res = try!(format(arg, FormatOp::from_char(cur), flags));
                            output.extend(res.iter().map(|x| *x));
                        } else {
                            return Err("stack is empty".to_string());
                        }
                    }
                    ':' | '#' | ' ' | '.' | '0'...'9' => {
                        let mut flags = Flags::new();
                        let mut fstate = FormatStateFlags;
                        match cur {
                            ':' => (),
                            '#' => flags.alternate = true,
                            ' ' => flags.space = true,
                            '.' => fstate = FormatStatePrecision,
                            '0'...'9' => {
                                flags.width = cur as usize - '0' as usize;
                                fstate = FormatStateWidth;
                            }
                            _ => unreachable!(),
                        }
                        state = FormatPattern(flags, fstate);
                    }

                    // conditionals
                    '?' => (),
                    't' => {
                        match stack.pop() {
                            Some(Number(0)) => state = SeekIfElse(0),
                            Some(Number(_)) => (),
                            Some(_) => {
                                return Err("non-number on stack with conditional".to_string())
                            }
                            None => return Err("stack is empty".to_string()),
                        }
                    }
                    'e' => state = SeekIfEnd(0),
                    ';' => (),
                    _ => return Err(format!("unrecognized format option {}", cur)),
                }
            }
            PushParam => {
                // params are 1-indexed
                stack.push(mparams[match cur.to_digit(10) {
                               Some(d) => d as usize - 1,
                               None => return Err("bad param number".to_string()),
                           }]
                           .clone());
            }
            SetVar => {
                if cur >= 'A' && cur <= 'Z' {
                    if let Some(arg) = stack.pop() {
                        let idx = (cur as u8) - b'A';
                        vars.sta[idx as usize] = arg;
                    } else {
                        return Err("stack is empty".to_string());
                    }
                } else if cur >= 'a' && cur <= 'z' {
                    if let Some(arg) = stack.pop() {
                        let idx = (cur as u8) - b'a';
                        vars.dyn[idx as usize] = arg;
                    } else {
                        return Err("stack is empty".to_string());
                    }
                } else {
                    return Err("bad variable name in %P".to_string());
                }
            }
            GetVar => {
                if cur >= 'A' && cur <= 'Z' {
                    let idx = (cur as u8) - b'A';
                    stack.push(vars.sta[idx as usize].clone());
                } else if cur >= 'a' && cur <= 'z' {
                    let idx = (cur as u8) - b'a';
                    stack.push(vars.dyn[idx as usize].clone());
                } else {
                    return Err("bad variable name in %g".to_string());
                }
            }
            CharConstant => {
                stack.push(Number(c as i32));
                state = CharClose;
            }
            CharClose => {
                if cur != '\'' {
                    return Err("malformed character constant".to_string());
                }
            }
            IntConstant(i) => {
                if cur == '}' {
                    stack.push(Number(i));
                    state = Nothing;
                } else if let Some(digit) = cur.to_digit(10) {
                    match i.checked_mul(10).and_then(|i_ten| i_ten.checked_add(digit as i32)) {
                        Some(i) => {
                            state = IntConstant(i);
                            old_state = Nothing;
                        }
                        None => return Err("int constant too large".to_string()),
                    }
                } else {
                    return Err("bad int constant".to_string());
                }
            }
            FormatPattern(ref mut flags, ref mut fstate) => {
                old_state = Nothing;
                match (*fstate, cur) {
                    (_, 'd') | (_, 'o') | (_, 'x') | (_, 'X') | (_, 's') => {
                        if let Some(arg) = stack.pop() {
                            let res = try!(format(arg, FormatOp::from_char(cur), *flags));
                            output.extend(res.iter().map(|x| *x));
                            // will cause state to go to Nothing
                            old_state = FormatPattern(*flags, *fstate);
                        } else {
                            return Err("stack is empty".to_string());
                        }
                    }
                    (FormatStateFlags, '#') => {
                        flags.alternate = true;
                    }
                    (FormatStateFlags, '-') => {
                        flags.left = true;
                    }
                    (FormatStateFlags, '+') => {
                        flags.sign = true;
                    }
                    (FormatStateFlags, ' ') => {
                        flags.space = true;
                    }
                    (FormatStateFlags, '0'...'9') => {
                        flags.width = cur as usize - '0' as usize;
                        *fstate = FormatStateWidth;
                    }
                    (FormatStateFlags, '.') => {
                        *fstate = FormatStatePrecision;
                    }
                    (FormatStateWidth, '0'...'9') => {
                        let old = flags.width;
                        flags.width = flags.width * 10 + (cur as usize - '0' as usize);
                        if flags.width < old {
                            return Err("format width overflow".to_string());
                        }
                    }
                    (FormatStateWidth, '.') => {
                        *fstate = FormatStatePrecision;
                    }
                    (FormatStatePrecision, '0'...'9') => {
                        let old = flags.precision;
                        flags.precision = flags.precision * 10 + (cur as usize - '0' as usize);
                        if flags.precision < old {
                            return Err("format precision overflow".to_string());
                        }
                    }
                    _ => return Err("invalid format specifier".to_string()),
                }
            }
            SeekIfElse(level) => {
                if cur == '%' {
                    state = SeekIfElsePercent(level);
                }
                old_state = Nothing;
            }
            SeekIfElsePercent(level) => {
                if cur == ';' {
                    if level == 0 {
                        state = Nothing;
                    } else {
                        state = SeekIfElse(level - 1);
                    }
                } else if cur == 'e' && level == 0 {
                    state = Nothing;
                } else if cur == '?' {
                    state = SeekIfElse(level + 1);
                } else {
                    state = SeekIfElse(level);
                }
            }
            SeekIfEnd(level) => {
                if cur == '%' {
                    state = SeekIfEndPercent(level);
                }
                old_state = Nothing;
            }
            SeekIfEndPercent(level) => {
                if cur == ';' {
                    if level == 0 {
                        state = Nothing;
                    } else {
                        state = SeekIfEnd(level - 1);
                    }
                } else if cur == '?' {
                    state = SeekIfEnd(level + 1);
                } else {
                    state = SeekIfEnd(level);
                }
            }
        }
        if state == old_state {
            state = Nothing;
        }
    }
    Ok(output)
}

#[derive(Copy, PartialEq, Clone)]
struct Flags {
    width: usize,
    precision: usize,
    alternate: bool,
    left: bool,
    sign: bool,
    space: bool,
}

impl Flags {
    fn new() -> Flags {
        Flags {
            width: 0,
            precision: 0,
            alternate: false,
            left: false,
            sign: false,
            space: false,
        }
    }
}

#[derive(Copy, Clone)]
enum FormatOp {
    FormatDigit,
    FormatOctal,
    FormatHex,
    FormatHEX,
    FormatString,
}

impl FormatOp {
    fn from_char(c: char) -> FormatOp {
        match c {
            'd' => FormatDigit,
            'o' => FormatOctal,
            'x' => FormatHex,
            'X' => FormatHEX,
            's' => FormatString,
            _ => panic!("bad FormatOp char"),
        }
    }
    fn to_char(self) -> char {
        match self {
            FormatDigit => 'd',
            FormatOctal => 'o',
            FormatHex => 'x',
            FormatHEX => 'X',
            FormatString => 's',
        }
    }
}

fn format(val: Param, op: FormatOp, flags: Flags) -> Result<Vec<u8>, String> {
    let mut s = match val {
        Number(d) => {
            match op {
                FormatDigit => {
                    if flags.sign {
                        format!("{:+01$}", d, flags.precision)
                    } else if d < 0 {
                        // C doesn't take sign into account in precision calculation.
                        format!("{:01$}", d, flags.precision + 1)
                    } else if flags.space {
                        format!(" {:01$}", d, flags.precision)
                    } else {
                        format!("{:01$}", d, flags.precision)
                    }
                }
                FormatOctal => {
                    if flags.alternate {
                        // Leading octal zero counts against precision.
                        format!("0{:01$o}", d, flags.precision.saturating_sub(1))
                    } else {
                        format!("{:01$o}", d, flags.precision)
                    }
                }
                FormatHex => {
                    if flags.alternate && d != 0 {
                        format!("0x{:01$x}", d, flags.precision)
                    } else {
                        format!("{:01$x}", d, flags.precision)
                    }
                }
                FormatHEX => {
                    if flags.alternate && d != 0 {
                        format!("0X{:01$X}", d, flags.precision)
                    } else {
                        format!("{:01$X}", d, flags.precision)
                    }
                }
                FormatString => return Err("non-number on stack with %s".to_string()),
            }
            .into_bytes()
        }
        Words(s) => {
            match op {
                FormatString => {
                    let mut s = s.into_bytes();
                    if flags.precision > 0 && flags.precision < s.len() {
                        s.truncate(flags.precision);
                    }
                    s
                }
                _ => return Err(format!("non-string on stack with %{}", op.to_char())),
            }
        }
    };
    if flags.width > s.len() {
        let n = flags.width - s.len();
        if flags.left {
            s.extend(repeat(b' ').take(n));
        } else {
            let mut s_ = Vec::with_capacity(flags.width);
            s_.extend(repeat(b' ').take(n));
            s_.extend(s.into_iter());
            s = s_;
        }
    }
    Ok(s)
}

#[cfg(test)]
mod test {
    use super::{expand, Variables};
    use super::Param::{self, Words, Number};
    use std::result::Result::Ok;

    #[test]
    fn test_basic_setabf() {
        let s = b"\\E[48;5;%p1%dm";
        assert_eq!(expand(s, &[Number(1)], &mut Variables::new()).unwrap(),
                   "\\E[48;5;1m".bytes().collect::<Vec<_>>());
    }

    #[test]
    fn test_multiple_int_constants() {
        assert_eq!(expand(b"%{1}%{2}%d%d", &[], &mut Variables::new()).unwrap(),
                   "21".bytes().collect::<Vec<_>>());
    }

    #[test]
    fn test_op_i() {
        let mut vars = Variables::new();
        assert_eq!(expand(b"%p1%d%p2%d%p3%d%i%p1%d%p2%d%p3%d",
                          &[Number(1), Number(2), Number(3)],
                          &mut vars),
                   Ok("123233".bytes().collect::<Vec<_>>()));
        assert_eq!(expand(b"%p1%d%p2%d%i%p1%d%p2%d", &[], &mut vars),
                   Ok("0011".bytes().collect::<Vec<_>>()));
    }

    #[test]
    fn test_param_stack_failure_conditions() {
        let mut varstruct = Variables::new();
        let vars = &mut varstruct;
        fn get_res(fmt: &str,
                   cap: &str,
                   params: &[Param],
                   vars: &mut Variables)
                   -> Result<Vec<u8>, String> {
            let mut u8v: Vec<_> = fmt.bytes().collect();
            u8v.extend(cap.as_bytes().iter().map(|&b| b));
            expand(&u8v, params, vars)
        }

        let caps = ["%d", "%c", "%s", "%Pa", "%l", "%!", "%~"];
        for &cap in caps.iter() {
            let res = get_res("", cap, &[], vars);
            assert!(res.is_err(),
                    "Op {} succeeded incorrectly with 0 stack entries",
                    cap);
            let p = if cap == "%s" || cap == "%l" {
                Words("foo".to_string())
            } else {
                Number(97)
            };
            let res = get_res("%p1", cap, &[p], vars);
            assert!(res.is_ok(),
                    "Op {} failed with 1 stack entry: {}",
                    cap,
                    res.err().unwrap());
        }
        let caps = ["%+", "%-", "%*", "%/", "%m", "%&", "%|", "%A", "%O"];
        for &cap in caps.iter() {
            let res = expand(cap.as_bytes(), &[], vars);
            assert!(res.is_err(),
                    "Binop {} succeeded incorrectly with 0 stack entries",
                    cap);
            let res = get_res("%{1}", cap, &[], vars);
            assert!(res.is_err(),
                    "Binop {} succeeded incorrectly with 1 stack entry",
                    cap);
            let res = get_res("%{1}%{2}", cap, &[], vars);
            assert!(res.is_ok(),
                    "Binop {} failed with 2 stack entries: {}",
                    cap,
                    res.err().unwrap());
        }
    }

    #[test]
    fn test_push_bad_param() {
        assert!(expand(b"%pa", &[], &mut Variables::new()).is_err());
    }

    #[test]
    fn test_comparison_ops() {
        let v = [('<', [1u8, 0u8, 0u8]), ('=', [0u8, 1u8, 0u8]), ('>', [0u8, 0u8, 1u8])];
        for &(op, bs) in v.iter() {
            let s = format!("%{{1}}%{{2}}%{}%d", op);
            let res = expand(s.as_bytes(), &[], &mut Variables::new());
            assert!(res.is_ok(), res.err().unwrap());
            assert_eq!(res.unwrap(), vec![b'0' + bs[0]]);
            let s = format!("%{{1}}%{{1}}%{}%d", op);
            let res = expand(s.as_bytes(), &[], &mut Variables::new());
            assert!(res.is_ok(), res.err().unwrap());
            assert_eq!(res.unwrap(), vec![b'0' + bs[1]]);
            let s = format!("%{{2}}%{{1}}%{}%d", op);
            let res = expand(s.as_bytes(), &[], &mut Variables::new());
            assert!(res.is_ok(), res.err().unwrap());
            assert_eq!(res.unwrap(), vec![b'0' + bs[2]]);
        }
    }

    #[test]
    fn test_conditionals() {
        let mut vars = Variables::new();
        let s = b"\\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m";
        let res = expand(s, &[Number(1)], &mut vars);
        assert!(res.is_ok(), res.err().unwrap());
        assert_eq!(res.unwrap(), "\\E[31m".bytes().collect::<Vec<_>>());
        let res = expand(s, &[Number(8)], &mut vars);
        assert!(res.is_ok(), res.err().unwrap());
        assert_eq!(res.unwrap(), "\\E[90m".bytes().collect::<Vec<_>>());
        let res = expand(s, &[Number(42)], &mut vars);
        assert!(res.is_ok(), res.err().unwrap());
        assert_eq!(res.unwrap(), "\\E[38;5;42m".bytes().collect::<Vec<_>>());
    }

    #[test]
    fn test_format() {
        let mut varstruct = Variables::new();
        let vars = &mut varstruct;
        assert_eq!(expand(b"%p1%s%p2%2s%p3%2s%p4%.2s",
                          &[Words("foo".to_string()),
                            Words("foo".to_string()),
                            Words("f".to_string()),
                            Words("foo".to_string())],
                          vars),
                   Ok("foofoo ffo".bytes().collect::<Vec<_>>()));
        assert_eq!(expand(b"%p1%:-4.2s", &[Words("foo".to_string())], vars),
                   Ok("fo  ".bytes().collect::<Vec<_>>()));

        assert_eq!(expand(b"%p1%d%p1%.3d%p1%5d%p1%:+d", &[Number(1)], vars),
                   Ok("1001    1+1".bytes().collect::<Vec<_>>()));
        assert_eq!(expand(b"%p1%o%p1%#o%p2%6.4x%p2%#6.4X",
                          &[Number(15), Number(27)],
                          vars),
                   Ok("17017  001b0X001B".bytes().collect::<Vec<_>>()));
    }
}
