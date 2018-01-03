extern crate term;

#[cfg(unix)]
#[test]
fn test_winsize() {
    let t = term::stdout().unwrap();
    // This makes sure we don't try to provide dims on an incorrect platform, it also may trigger
    // any memory errors.
    let _dims = t.dims().unwrap();
}
