use std::{env, process::exit};
extern crate fits;

fn main() {
    let mut args = env::args();
    let filename = if let Some(arg) = args.nth(1) {arg} else {
        println!("Please give a filename as the first argument");
        exit(1);
    };
    if let Some(fits) = fits::BasicFits::open(&filename) {
        let h = fits.header;

        h.print_keywords();
        println!("");
        println!("-------Extracted: ");
        println!("SIMPLE {}", h.simple);
        println!("BITPIX {:?}", h.bitpix);
        println!("NAXIS  {}", h.naxis);
        println!("Axes   {:?}", h.axes);

        if let Some(data) = fits.data {
            let sum: f64 = data.iter().sum::<f64>();
            let avg: f64 = sum / data.len() as f64;

            println!("-------Data Stuff:");
            println!("Sum: {:.2e}", sum);
            println!("Avg: {:.2e}", avg);
        }
    } else {
        println!("Something went wrong while reading the file {}...", filename);
        exit(1);
    }
}


