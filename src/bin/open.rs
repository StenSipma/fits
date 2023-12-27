use std::{env, process::exit};
extern crate fits;

fn main() {
    let mut args = env::args();
    let filename = if let Some(arg) = args.nth(1) {
        arg
    } else {
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
        println!("NAXIS  {}", h.naxis.get());
        println!("Axes   {:?}", h.axes);

        let data = &fits.data;
        let sum: f64 = data.into_iter().sum::<f64>();
        let avg: f64 = sum / data.len() as f64;
        let rem = data - avg;
        let var: f64 = (&rem * &rem).into_iter().sum::<f64>() / data.len() as f64;

        println!("-------Data Stuff:");
        println!("Sum: {:.2e}", sum);
        println!("Avg: {:.2e}", avg);
        println!("Std: {:.2e}", var.sqrt());
    } else {
        println!(
            "Something went wrong while reading the file {}...",
            filename
        );
        exit(1);
    }
}
