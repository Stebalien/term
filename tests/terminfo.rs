extern crate term;

use term::terminfo::TermInfo;
use std::fs;

#[test]
fn test_parse() {
    for f in fs::read_dir("tests/data/").unwrap() {
        let _ = TermInfo::from_path(f.unwrap().path()).unwrap();
    }
}
