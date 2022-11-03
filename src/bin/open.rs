use std::env;
extern crate fits;

fn main() {
    let mut args = env::args();
    let filename = if let Some(arg) = args.nth(1) {arg} else {"../python-fits/simple.fits".to_string()};
    if let Some((h, b)) = fits::Fits::open(filename) {
        fits::parsing::print_header(&h);
        let (naxis, axes, bitpix) = fits::parsing::header::extract_values(&h);
        // temporary print
        println!("");
        println!("-------Extracted: ");
        println!("NAXIS {}", naxis);
        println!("BITPIX {}", bitpix);
        println!("axes {:?}", axes);

        if let Some(b) = b {
            let sum: f64 = b.iter().sum::<f64>();
            let avg: f64 = sum / b.len() as f64;

            println!("-------Data Stuff:");
            println!("Sum: {:.2e}", sum);
            println!("Avg: {:.2e}", avg);
        }
    }
}


