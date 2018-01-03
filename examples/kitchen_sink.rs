// An example of the avaiable functionality in term

extern crate term;

fn main() {
    let mut t = term::stdout().unwrap();

    print!("Dims: ");
    t.fg(term::color::GREEN).unwrap();
    print!("{:?}", t.dims().unwrap());
    t.reset().unwrap();
    println!();
}
