use std::{env, process::exit};

extern crate fits;
use ndarray::Array2;
use viuer::Config;

mod image_util {
    use tightness::bound;
    use ndarray::Array2;

    const GRAY_MAP: [char; 69] = ['$', '@', 'B', '%', '8', '&', 'W', 'M', '#', '*', 'o', 'a', 'h', 'k', 'b', 'd', 'p', 'q', 'w', 'm', 'Z', 'O', '0', 'Q', 'L', 'C', 'J', 'U', 'Y', 'X', 'z', 'c', 'v', 'u', 'n', 'x', 'r', 'j', 'f', 't', '/', '\\', '|', '(', ')', '1', '{', '}', '[', ']', '?', '-', '_', '+', '~', '<', '>', 'i', '!', 'l', 'I', ';', ':', ',', '"', '^', '`', '\'', '.', ];
    static N: f64 = GRAY_MAP.len() as f64;
    const MAX_GRAY: f64 = 255.;
    const MIN_GRAY: f64 = 0.;

    bound!(pub GrayValue: f64 where |n| *n >= MIN_GRAY && *n <= MAX_GRAY);

    pub fn float_to_ascii(val: GrayValue) -> char {
        // Map val to an integer in the interval [0, N)
        // Check if N-1 is indeed correct
        let idx = (val.get() * (N - 1.) ).floor() as usize;
        // Get the character at location idx
        GRAY_MAP[idx]
    }

    // Normalized the f64 array to be between MIN_GRAY and MAX_GRAY
    pub fn normalize(data: &Array2<f64>) -> Array2<f64> {
        let max = data.iter().fold(f64::MAX, |a, &b| a.min(b));
        let min = data.iter().fold(f64::MIN, |a, &b| a.max(b));
        let norm = (data - min) / (max - min);
        norm*(MAX_GRAY - MIN_GRAY) + MIN_GRAY
    }

}

const MAX_VALUE: usize = 80;

fn plot_image_term(data: &Array2<f64>) {
    let norm = image_util::normalize(data);
    let byte_image: Vec<u8> = norm.iter().map(|n| *n as u8).collect();

    let shape = data.shape();
    let im = image::GrayImage::from_vec(shape[0] as u32, shape[1] as u32, byte_image)
        .expect("Creation of image failed");
    let img = image::DynamicImage::ImageLuma8(im);

    let conf = Config {
        // set offset
        x: 0,
        y: 20,
        // set dimensions
        width: Some(80),
        height: Some(20),
        ..Default::default()
    };
    viuer::print(&img, &conf).expect("Image printing failed.");
}

fn plot_image_ascii(data: &Array2<f64>) {
    println!("{:?}", data);

    let norm = image_util::normalize(data);
    let chars: Vec<char> = norm
        .iter()
        .cloned()
        .map(|n| image_util::float_to_ascii(image_util::GrayValue::new(n).unwrap()))
        .collect();
    let char_array = Array2::from_shape_vec(norm.dim(), chars).unwrap();
    for row in char_array.outer_iter() {
        for cr in row.iter() {
            print!("{}", cr);
        }
        print!("\n");
    }

    // println!("{:?}", char_array);
}

fn clamp<T: Ord>(n: T, min: T, max: T) -> T {
    n.min(max).max(min)
}

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

        // h.print_keywords();
        println!("File {}: ", filename);
        println!(" ");
        // println!("SIMPLE {}", h.simple);
        // println!("BITPIX {:?}", h.bitpix);
        println!("NAXIS  {}", h.naxis.get());
        println!("Axes   {:?}", h.axes);

        // Calculate some basic statistics of the data:
        let data = &fits.data;
        let sum: f64 = data.into_iter().sum::<f64>();
        let avg: f64 = sum / data.len() as f64;
        let rem = data - avg;
        let var: f64 = (&rem * &rem).into_iter().sum::<f64>() / data.len() as f64;
        let min = data.fold(f64::MAX, |a, &b| a.min(b));
        let max = data.fold(f64::MIN, |a, &b| a.max(b));

        println!("-------Data Stuff:");
        println!("Sum: {:.2e}", sum);
        println!("Avg: {:.2e}", avg);
        println!("Std: {:.2e}", var.sqrt());
        println!("Min / max : {} / {}", min, max);
        println!("IMAGE:");


        if *h.naxis.get() == 2 {
            let axis = (h.axes[0], h.axes[1]);
            let data2d = data.clone().into_shape(axis).unwrap();
            
            // let data2d = data2d; // normalize to 0
                                       
            // TODO: find a nice scheme to automatically normalize the image
            // something like ZScale (is complicated), or cutting percentiles (requires)
            // a histogram implementation.
            let vmin = 1000.;
            let vmax = 10000.;
            let data2d = data2d.map(|e| e.clamp(vmin, vmax));
            let data2d = data2d.map(|x| (1. + x).log10()); // Log1p

            plot_image_term(&data2d);
        }
    } else {
        println!(
            "Something went wrong while reading the file {}...",
            filename
        );
        exit(1);
    }
}

