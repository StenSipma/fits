// Link: https://fits.gsfc.nasa.gov/standard40/fits_standard40aa-le.pdf

#[allow(dead_code)]
mod definitions {
    pub const BLOCK_SIZE: usize = 2880; // bytes per block

    // Header sizes
    pub const HEADER_KEYWORD_SIZE: usize = 80; // characters per header keyword
    pub const HEADER_KEYWORD_NAME_SIZE: usize = 8; // the length (#chars) of the keyword name (i.e. NAXIS)
    pub const HEADER_VALUE_INDICATOR_SIZE: usize = 2; // the length (#chars) of the value indicator (i.e. '= ')
    pub const HEADER_VALUE_SIZE: usize = 70; // the length (#chars) of the value in a keyword

    pub const HEADER_VALUE_INDICATOR: &str = "= ";

    // Specific header keywords
    pub const HEADER_END_KEYWORD: &str = "END";
    pub const HEADER_HISTORY_KEYWORD: &str = "HISTORY";
    pub const HEADER_COMMENT_KEYWORD: &str = "COMMENT";
    pub const HEADER_CONTINUE_KEYWORD: &str = "CONTINUE";

    // FITS with only a primary HDU is a 'Basic FITS File' or a 'Single Image FITS (SIF) File'
    // FITS with one or more extensions is a Multi-Extension FITS (MEF) file .
}

#[allow(dead_code)]
mod parsing_old {
    use std::slice::Chunks;
    use std::str::Utf8Error;
    use std::{str, fmt};

    use crate::{definitions, HeaderList};

    fn print_header(header: &Vec<header::Keyword>) {
        for keyword in header.into_iter() {
            keyword.print()
        }
    }

    pub mod header {
        use super::*;

        #[derive(PartialEq)]
        #[derive(Debug)]

        pub enum Keyword<'a> {
            End,
            History(&'a str),
            Comment(&'a str),
            Continue(&'a str),
            RawValue(&'a str, &'a str), // we have to process this later into the specific type
            ParsedValue(&'a str, Value, &'a str) // This is the processed version, with a possible comment
        }

        impl<'a> Keyword<'a> {
            pub fn print(&self) {
                // This is just a basic print function, mainly for a bit better debugging
                match self {
                    Keyword::ParsedValue(kw, value, comment) => println!("{:8} | {:>30} / {}", kw, value, comment),
                    Keyword::RawValue(kw, value) => println!("{:8} | {:>30}", kw, value),
                    Keyword::History(v)          => println!("{:8} {:>30}", definitions::HEADER_HISTORY_KEYWORD, v),
                    Keyword::Comment(v)          => println!("{:8} {:>30}", definitions::HEADER_COMMENT_KEYWORD, v),
                    Keyword::Continue(v)         => println!("{:8} {:>30}", definitions::HEADER_CONTINUE_KEYWORD, v),
                    Keyword::End                 => println!("{:8}", definitions::HEADER_END_KEYWORD),
                }
            }

            pub fn parse_from_bytes(keyword_bytes: &'a [u8]) -> Result<Keyword<'a>, Utf8Error> {
                let keyword = str::from_utf8(keyword_bytes.into())?;
                let (kw, _sep, value) = split_keyword(keyword);

                let kw = kw.trim_matches(' ');
                let value = value.trim_matches(' ');

                Ok (
                    match kw {
                    definitions::HEADER_END_KEYWORD => {Keyword::End},
                    definitions::HEADER_COMMENT_KEYWORD => {Keyword::Comment(value)},
                    definitions::HEADER_CONTINUE_KEYWORD => {Keyword::Continue(value)},
                    definitions::HEADER_HISTORY_KEYWORD => {Keyword::History(value)},
                    kw => {Keyword::RawValue(kw, value)}
                })
            }
        }

        fn split_keyword<'a>(keyword: &'a str) -> (&'a str, &'a str, &'a str) {
            // NOTE: keyword MUST have 80 characters.
            let name_idx = definitions::HEADER_KEYWORD_NAME_SIZE;
            let sep_idx = name_idx + definitions::HEADER_VALUE_INDICATOR_SIZE;

            // May or may not have a value indicator
            if &keyword[name_idx..sep_idx] == definitions::HEADER_VALUE_INDICATOR {
                (&keyword[..name_idx], &keyword[name_idx..sep_idx], &keyword[sep_idx..])
            } else {
                (&keyword[..name_idx], "", &keyword[name_idx..])
            }

        }

        #[derive(PartialEq, Debug)]
        pub enum Value {
            Undefined,
            Integer(i64),
            Str(String),
            Float(f64),
            Boolean(bool),
            // TODO: Add complex integers and complex floats
        }

        impl fmt::Display for Value {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    Value::Undefined => {write!(f, "")},
                    Value::Integer(x) => {write!(f, "{}", x)},
                    Value::Float(x) => {write!(f, "{}", x)},
                    Value::Str(x) => {write!(f, "'{}'", x)},
                    Value::Boolean(x) => {
                        write!(f, "{}", if *x {"T"} else {"F"})},
                }
                
            }
        }

        impl Value {
            pub fn from_str(value: &str) -> Self {
                if value.is_empty() {
                    return Value::Undefined;
                }

                let value_bytes = value.as_bytes();

                // Only a comment
                if value_bytes[0] == b'/' {
                    return Value::Undefined;
                }

                if value_bytes[0] == b'\'' {
                    let mut extracted: Vec<u8> = Vec::new();
                    extract_str(value_bytes, &mut extracted);
                    return Value::Str(String::from_utf8(extracted).unwrap())
                }

                if value_bytes[0] == b'T' || value_bytes[0] == b'F' {
                    return Value::Boolean(value_bytes[0] == b'T');
                }

                let (pre_comment, _after)  = match value.split_once(&[' ', '/'][..]) {
                    Some((a, b)) => {(a, b)},
                    None => {(value, "")},
                };
                if pre_comment.find('.').is_some() {
                    let num = pre_comment.parse().unwrap();
                    return Value::Float(num);
                }

                if pre_comment.chars().all(|x| x.is_numeric() || x == '-' || x == '+' ) {
                    let num = pre_comment.parse().unwrap();
                    return Value::Integer(num);
                }

                Value::Undefined
            }

            fn from_value(v: &Value) -> Value {
                match v {
                    Value::Boolean(n) => Value::Boolean(*n),
                    Value::Integer(n) => Value::Integer(*n),
                    Value::Float(n) => Value::Float(*n),
                    Value::Undefined => Value::Undefined,
                    Value::Str(s) => Value::Str(s.clone()),
                }
            }
        }

        fn extract_str<'a>(input: &'a [u8], output: &mut Vec<u8>) {
            // We know input[0] == b'\''
            let mut i:usize = 1;
            while i < input.len() {
                if input[i] == b'\'' {
                    if i+1 < input.len() && input[i+1] == b'\'' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                i += 1;
            }
            let extract = &input[1..i];

            let mut prev = b' ';
            // let mut bytes: Vec<u8> = Vec::new();
            for c in extract {
                if !(*c == b'\'' && prev == b'\'') {
                    output.push(*c);
                }
                prev = *c;
            }
        }


        pub fn parse_header<'a>(blocks: &mut Chunks<'a, u8>) -> HeaderList<'a> {
            let mut header: HeaderList = Vec::new();
            let mut reading_header = true;

            while reading_header {
                let block = blocks.next().unwrap();

                for keyword_bytes in block.chunks(definitions::HEADER_KEYWORD_SIZE) {
                    match Keyword::parse_from_bytes(keyword_bytes).unwrap() {
                        Keyword::End => {
                            reading_header = false;
                            break;
                        },
                        Keyword::RawValue(kw, rv) => {
                            let val = Value::from_str(rv);
                            header.push(Keyword::ParsedValue(kw, val, ""))
                        }
                        keyword => {
                            header.push(keyword)
                        }
                    };
                }
            }
            header
        }

        fn find_raw_value<'a, 'b>(header: &HeaderList<'a>, key: &'b str) -> Option<&'a str> {
            for kw in header.iter() {
                if let Keyword::RawValue(k, v) = kw {
                    if *k == key {
                        return Some(v);
                    }
                }
            }
            None
        }

        fn find_value<'a, 'b>(header: &'a HeaderList<'a>, key: &'b str) -> Option<Value> {
            for kw in header.iter() {
                match kw {
                    Keyword::RawValue(k, v) => {
                        if *k == key {
                            return Some(Value::from_str(v))
                        }
                    },
                    Keyword::ParsedValue(k, v, _) => {
                        if *k == key {
                            return Some(Value::from_value(v))
                        }
                    },
                    _ => { continue; }
                };
            };
            None
        }

        pub fn extract_values(header: &HeaderList) -> (usize, Vec<usize>, i64) {
            let naxis = {
                let value_naxis = find_value(header, "NAXIS").unwrap();
                if let Value::Integer(i) = value_naxis {
                    i as usize
                } else {
                    panic!("Naxis was not an integer");
                }
            };

            let bitpix = {
                let value_bitpix = find_value(header, "BITPIX").unwrap();
                if let Value::Integer(i) = value_bitpix {
                    i
                } else {
                    panic!("BITPIX was not an integer");
                }
            };

            let mut axes = Vec::with_capacity(naxis);
            for i in 1..=naxis {
                let kw = format!("NAXIS{}", i);
                let kw = kw.as_str();
                let value_axis = find_value(header, kw).unwrap();
                if let Value::Integer(i) = value_axis {
                    axes.push(i as usize);
                } else {
                    panic!("{} was not an integer", kw);
                }
            }

            (naxis, axes, bitpix)
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn parse_from_bytes_test() {
                // Full keyword
                let keyword = "SIMPLE  =                    T / conforms to FITS standard                      ";
                let keyword_bytes = Vec::from_iter(keyword.bytes());
                let res = Keyword::parse_from_bytes(&keyword_bytes);
                assert_eq!(res.unwrap(), Keyword::RawValue("SIMPLE", "T / conforms to FITS standard"));

                // No comment
                let keyword = "SIMPLE  =                    T                                                  ";
                let keyword_bytes = Vec::from_iter(keyword.bytes());
                let res = Keyword::parse_from_bytes(&keyword_bytes);
                assert_eq!(res.unwrap(), Keyword::RawValue("SIMPLE", "T"));

                // No value separator
                let keyword = "COMMENT This is a comment, and therefore does not have a value separator.       ";
                let keyword_bytes = Vec::from_iter(keyword.bytes());
                let res = Keyword::parse_from_bytes(&keyword_bytes);
                assert_eq!(res.unwrap(), Keyword::Comment("This is a comment, and therefore does not have a value separator."));

                // End keyword
                let keyword = "END                                                                             ";
                let keyword_bytes = Vec::from_iter(keyword.bytes());
                let res = Keyword::parse_from_bytes(&keyword_bytes);
                assert_eq!(res.unwrap(), Keyword::End);

                // Should fail:
                // Unexpected '/' in the string value
                let keyword = "KEYWORD =                       'something with a /       ' / and also a comment";
                let keyword_bytes = Vec::from_iter(keyword.bytes());
                let res = Keyword::parse_from_bytes(&keyword_bytes);
                assert_eq!(res.unwrap(), Keyword::RawValue("KEYWORD", "'something with a /       ' / and also a comment"));

                let _tmp = "SIMPLE  =                    T / conforms to FITS standard                      BITPIX  =                  -64 / array data type                                NAXIS   =                    2 / number of array dimensions                     NAXIS1  =                 1024                                                  NAXIS2  =                  682                                                  BIAS    =                  100                                                  FOCALLEN= +0.000000000000E+000                                                  APTAREA = +0.000000000000E+000                                                  APTDIA  = +0.000000000000E+000                                                  DATE-OBS= '2020-04-18T00:56:58.604'                                             TIME-OBS= '00:56:58.604        '                                                SWCREATE= 'CCDSoft Version 5.00.218'                                            SET-TEMP= -2.000000000000E+001                                                  COLORCCD=                    0                                                  DISPCOLR=                    1                                                  IMAGETYP= 'Light Frame         '                                                CCDSFPT =                    1                                                  XORGSUBF=                    0                                                  YORGSUBF=                    0                                                  CCDSUBFL=                    0                                                  CCDSUBFT=                    0                                                  XBINNING=                    3                                                  CCDXBIN =                    3                                                  YBINNING=                    3                                                  CCDYBIN =                    3                                                  EXPSTATE=                  293                                                  CCD-TEMP= -2.041762134545E+001                                                  TEMPERAT= -2.041762134545E+001                                                  OBJECT  = 'Entered_Coordinates '                                                OBJCTRA = '14 49 09.474        '                                                OBJCTDEC= '+40 42 04.35        '                                                TELTKRA = -1.000000000000E+003                                                  TELTKDEC= -1.000000000000E+003                                                  CENTAZ  = +1.966280653172E+002                                                  CENTALT = +7.695155713274E+001                                                  TELHA   = '00 20 20.742        '                                                LST     = '15 09 30.056        '                                                AIRMASS = +1.026504260005E+000                                                  SITELAT = '+53:14:24.90        '                                                SITELONG= '-006:32:11.02       '                                                INSTRUME= 'SBIG STL-6303 3 CCD Camera'                                          EGAIN   = +2.360000000000E+000                                                  E-GAIN  = +2.360000000000E+000                                                  XPIXSZ  = +2.700000000000E+001                                                  YPIXSZ  = +2.700000000000E+001                                                  SBIGIMG =                   18                                                  USER_2  = 'SBIG STL-6303 3 CCD Camera'                                          DATAMAX =                65535                                                  SBSTDVER= 'SBFITSEXT Version 1.0'                                               FILTER  = 'R                   '                                                EXPTIME = +3.000000000000E+002                                                  EXPOSURE= +3.000000000000E+002                                                  CBLACK  =                 3754                                                  CWHITE  =                 4141                                                  END                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             ";
            }

            #[test]
            fn extract_str_test() {
                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'Hello'", &mut out);
                assert_eq!(out, b"Hello");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'a'", &mut out);
                assert_eq!(out, b"a");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'something'/comment", &mut out);
                assert_eq!(out, b"something");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'something' / comment", &mut out);
                assert_eq!(out, b"something");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'something   ' / comment", &mut out);
                assert_eq!(out, b"something   ");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'''' / comment", &mut out);
                assert_eq!(out, b"'", "Singular quote");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"'' / comment", &mut out);
                assert_eq!(out, b"");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"''", &mut out);
                assert_eq!(out, b"");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"' / is string ' / and comment", &mut out);
                assert_eq!(out, b" / is string ");

                let mut out: Vec<u8> = Vec::new();
                extract_str(b"' / is string ' / and comment ' ''' with quote", &mut out);
                assert_eq!(out, b" / is string ");
            }
        }
    }

    mod data {
        use super::*;
        use std::slice::Chunks;

        pub fn chuncks_to_data_f64<'a>(blocks: &mut Chunks<'a, u8>, size: usize, bytes: u64) -> Vec<f64> {
            // NOTE: assume we are reading '64' floats:
            let mut data: Vec<f64> = Vec::with_capacity(size);
            let mut rem = bytes;
            while rem > 0 {
                let block = blocks.next().unwrap();
                let read = rem.min(definitions::BLOCK_SIZE as u64);
                for (x, _i) in block.chunks_exact((64/8) as usize).zip(0..read/8) {
                    // make of exact size
                    let bts = [x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]];
                    let n = f64::from_be_bytes(bts);
                    data.push(n);
                }
                rem -= read;
            }
            data
        }
    }

    pub fn read_fits_buffer(buffer: Vec<u8>) -> Option<Vec<f64>> {
        let mut blocks = buffer.chunks(definitions::BLOCK_SIZE);

        // Read header (PrimaryHDU) must always exist
        let header = header::parse_header(&mut blocks);

        // temporary print
        print_header(&header);

        let (naxis, axes, bitpix) = header::extract_values(&header);
        println!("Extracted: ");
        println!("NAXIS {}", naxis);
        println!("BITPIX {}", bitpix);
        println!("axes {:?}", axes);

        // Potentially convert the header to a HashMap
        // Check if the data unit exists for the PrimaryHDU (look at NAXIS)
        
        // Calculate the total number of bytes
        let bytes: u64 = (axes.iter().product::<usize>() as u64 * (bitpix.abs() as u64)) / 8;
        println!("Total bytes: {}", bytes);
        let size = axes.iter().product::<usize>();

        if bitpix == -64 {
            let data = data::chuncks_to_data_f64(&mut blocks, size, bytes);
            // Move the parsed data into the array
            // let arr = Array::from_vec(data);
            // let arr = arr.into_shape(axes).unwrap();

            // Print some random things
            // println!("{:?} {} {}", arr.shape(), arr.sum(), arr.mean().unwrap());
            return Some(data)
        } else {
            println!("Other data format; bitpix {}", bitpix);
            None
        }
    }

}

#[allow(dead_code)]
mod parsing;

use std::io::Read;
use std::fs::File;

type HeaderList<'a> = Vec<parsing_old::header::Keyword<'a>>;
pub struct Fits {}

impl Fits {
    pub fn open(filename: String) -> Option<Vec<f64>> {
        let mut f = File::open(filename).unwrap();

        let mut buffer = Vec::new();

        if let Ok(_) = f.read_to_end(&mut buffer) {
            parsing_old::read_fits_buffer(buffer)
        } else {
            None
        }
    }
}
