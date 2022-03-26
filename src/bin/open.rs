use std::env;
extern crate fits;

fn main() {
    let mut args = env::args();
    let filename = if let Some(arg) = args.nth(1) {arg} else {"../python-fits/simple.fits".to_string()};
    let _fits = fits::Fits::open(filename);
}


