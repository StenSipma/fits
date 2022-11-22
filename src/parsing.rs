use std::slice::Chunks;
use std::str::Utf8Error;
use std::{fmt, str};
use tensor::Tensor;

use crate::header::Header;
use crate::{definitions, KeywordList, RawHeaderList};

pub mod header {
    use crate::definitions::HEADER_CONTINUE_KEYWORD;

    use super::*;

    pub enum Keyword {
        History(String),
        Comment(String),
        Value(String, Value, String),
        Continue(String, Value, String),
    }

    impl Keyword {
        pub fn print(&self) {
            // This is just a basic print function, mainly for a bit better debugging
            match self {
                Keyword::Value(kw, value, comment) => {
                    println!("{:8} | {:>30} / {}", kw, value, comment)
                }
                Keyword::Continue(kw, value, comment) => {
                    println!("{:8} | {:>30} / {}", kw, value, comment)
                }
                Keyword::History(v) => {
                    println!("{:8} {:>30}", definitions::HEADER_HISTORY_KEYWORD, v)
                }
                Keyword::Comment(v) => {
                    println!("{:8} {:>30}", definitions::HEADER_COMMENT_KEYWORD, v)
                }
            }
        }
    }

    #[derive(PartialEq, Debug)]
    pub enum HeaderChunk<'a> {
        End,
        History(&'a str),
        Comment(&'a str),
        RawValue(&'a str, &'a str),
    }

    impl<'a> HeaderChunk<'a> {
        pub fn print(&self) {
            // This is just a basic print function, mainly for a bit better debugging
            match self {
                // RawKeyword::ParsedValue(kw, value, comment) => println!("{:8} | {:>30} / {}", kw, value, comment),
                HeaderChunk::RawValue(kw, value) => println!("{:8} | {:>30}", kw, value),
                HeaderChunk::History(v) => {
                    println!("{:8} {:>30}", definitions::HEADER_HISTORY_KEYWORD, v)
                }
                HeaderChunk::Comment(v) => {
                    println!("{:8} {:>30}", definitions::HEADER_COMMENT_KEYWORD, v)
                }
                HeaderChunk::End => println!("{:8}", definitions::HEADER_END_KEYWORD),
            }
        }

        pub fn from_bytes(hc_bytes: &'a [u8]) -> Result<HeaderChunk<'a>, Utf8Error> {
            if hc_bytes == definitions::HEADER_END_KEYWORD_FULL {
                return Ok(HeaderChunk::End);
            }
            let chunk = str::from_utf8(hc_bytes.into())?;
            let (kw, _sep, value) = split_header_chunk(chunk);

            let kw = kw.trim_matches(' ');
            let value = value.trim_matches(' ');

            Ok(match kw {
                definitions::HEADER_COMMENT_KEYWORD => HeaderChunk::Comment(value),
                definitions::HEADER_HISTORY_KEYWORD => HeaderChunk::History(value),
                kw => HeaderChunk::RawValue(kw, value),
            })
        }

        pub fn parse(&self) -> Keyword {
            match self {
                Self::End => panic!("Should be no end value ever."),
                Self::History(v) => Keyword::History(v.to_string()),
                Self::Comment(v) => Keyword::Comment(v.to_string()),
                Self::RawValue(kw, value) => {
                    let val = Value::from_str(value);
                    // TODO: Parse comment as well.
                    // TODO: Check if we have a Continue thing
                    Keyword::Value(kw.to_string(), val, String::new())
                }
            }
        }
    }

    fn split_header_chunk<'a>(header_chunk: &'a str) -> (&'a str, &'a str, &'a str) {
        // NOTE: a single chunk MUST have 80 characters.
        let name_idx = definitions::HEADER_KEYWORD_NAME_SIZE;
        let sep_idx = name_idx + definitions::HEADER_VALUE_INDICATOR_SIZE;

        // May or may not have a value indicator
        if &header_chunk[name_idx..sep_idx] == definitions::HEADER_VALUE_INDICATOR {
            (
                &header_chunk[..name_idx],
                &header_chunk[name_idx..sep_idx],
                &header_chunk[sep_idx..],
            )
        } else {
            (&header_chunk[..name_idx], "", &header_chunk[name_idx..])
        }
    }

    #[derive(PartialEq, Debug, Clone)]
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
                Value::Undefined => {
                    write!(f, "")
                }
                Value::Integer(x) => {
                    write!(f, "{}", x)
                }
                Value::Float(x) => {
                    write!(f, "{}", x)
                }
                Value::Str(x) => {
                    write!(f, "'{}'", x)
                }
                Value::Boolean(x) => {
                    write!(f, "{}", if *x { "T" } else { "F" })
                }
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
                return Value::Str(String::from_utf8(extracted).unwrap());
            }

            if value_bytes[0] == b'T' || value_bytes[0] == b'F' {
                return Value::Boolean(value_bytes[0] == b'T');
            }

            let (pre_comment, _after) = match value.split_once(&[' ', '/'][..]) {
                Some((a, b)) => (a, b),
                None => (value, ""),
            };
            if pre_comment.find('.').is_some() {
                let num = pre_comment.parse().unwrap();
                return Value::Float(num);
            }

            if pre_comment
                .chars()
                .all(|x| x.is_numeric() || x == '-' || x == '+')
            {
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

        // Simple checking for what kind of type the value is.
        pub fn is_undefined(&self) -> bool {
            match self {
                Self::Undefined => true,
                _ => false,
            }
        }

        pub fn is_float(&self) -> bool {
            match self {
                Self::Float(_) => true,
                _ => false,
            }
        }

        pub fn is_boolean(&self) -> bool {
            match self {
                Self::Boolean(_) => true,
                _ => false,
            }
        }

        pub fn is_integer(&self) -> bool {
            match self {
                Self::Integer(_) => true,
                _ => false,
            }
        }

        pub fn is_str(&self) -> bool {
            match self {
                Self::Str(_) => true,
                _ => false,
            }
        }
    }

    fn parse_keyword(line: &str) -> (Value, String) {
        if line.is_empty() {
            return (Value::Undefined, String::new());
        }

        // Convert to bytes so we can index
        // let _value_bytes = line.as_bytes();

        // Case we have a string
        if line.starts_with("'") {
            todo!("Extract string & comment value");
        }
        // if value_bytes[0] == b'\'' {
        //     let mut extracted: Vec<u8> = Vec::new();
        //     extract_str(value_bytes, &mut extracted);
        //     return Value::Str(String::from_utf8(extracted).unwrap());
        // }

        // TODO: Verify that this split is correct.
        let (value, comment) = match line.split_once([' ', '/']) {
            Some((a, b)) => (a, b),
            None => (line, ""),
        };

        // Case of only a comment
        if value.is_empty() {
            return (Value::Undefined, comment.to_string());
        }

        // Case of a boolean
        if value.starts_with(['T', 'F']) {
            return (Value::Boolean(value.starts_with("T")), comment.to_string());
        }

        // Case of a complex number
        if value.starts_with('(') {
            todo!("Implement complex numbers")
        }

        // Case of a exponent
        // Case of a float
        if value.find(['.', 'E', 'D']).is_some() {
            let num = value.parse().unwrap();
            return (Value::Float(num), comment.to_string());
        }

        // Case of a integer
        if value
            .chars()
            .all(|x| x.is_numeric() || x == '-' || x == '+')
        {
            let num = value.parse().unwrap();
            return (Value::Integer(num), comment.to_string());
        }

        // No case matched
        (Value::Undefined, String::new())
    }

    fn extract_str<'a>(input: &'a [u8], output: &mut Vec<u8>) {
        // We know input[0] == b'\''
        let mut i: usize = 1;
        while i < input.len() {
            if input[i] == b'\'' {
                if i + 1 < input.len() && input[i + 1] == b'\'' {
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

    pub fn parse_header<'a>(blocks: &mut Chunks<'a, u8>) -> KeywordList {
        let mut raw_header: RawHeaderList = Vec::new();
        let mut reading_header = true;

        // First read the raw header
        while reading_header {
            let block = blocks.next().unwrap();

            for header_chunk_bytes in block.chunks(definitions::HEADER_KEYWORD_SIZE) {
                match HeaderChunk::from_bytes(header_chunk_bytes).unwrap() {
                    HeaderChunk::End => {
                        reading_header = false;
                        break;
                    }
                    header_chunk => raw_header.push(header_chunk),
                };
            }
        }

        // Turn into a parsed header
        let mut header: KeywordList = Vec::new();
        let continue_kw = HEADER_CONTINUE_KEYWORD.to_string();
        for chunk in raw_header.into_iter() {
            let parsed = chunk.parse();

            // Merge continue keywords into a single value keyword
            match parsed {
                Keyword::Value(kw, v0, c0) if kw == continue_kw && v0.is_str() => {
                    match header.pop() {
                        Some(Keyword::Value(kw, Value::Str(mut s), mut c)) if s.ends_with("&") => {
                            let v0 = if let Value::Str(v0) = v0 {
                                v0
                            } else {
                                panic!("CONTINUE Keyword did not have a string");
                            };
                            s.pop(); // remove the last &
                            c.pop(); // remove the last &
                            s.push_str(&v0);
                            c.push_str(&c0);
                            let new = Keyword::Value(kw, Value::Str(s), c);
                            header.push(new);
                        }
                        Some(prev) => {
                            // TODO: Maybe print some warning here, since we have a CONTINUE
                            // as keyword.
                            header.push(prev);
                            header.push(Keyword::Continue(
                                definitions::HEADER_CONTINUE_KEYWORD.to_string(),
                                v0,
                                c0,
                            ));
                        }
                        None => header.push(Keyword::Continue(
                            definitions::HEADER_CONTINUE_KEYWORD.to_string(),
                            v0,
                            c0,
                        )),
                    }
                }
                kw => header.push(kw),
            }
        }
        header
    }

    // TODO: Move as method of a proper Header datatype
    fn find_value<'a, 'b>(header: &'a KeywordList, key: &'b str) -> Option<Value> {
        for kw in header.iter() {
            match kw {
                Keyword::Value(k, v, _c) => {
                    if *k == key {
                        return Some(v.clone());
                    }
                }
                _ => {
                    continue;
                }
            };
        }
        None
    }

    pub fn extract_values(header: &KeywordList) -> (bool, usize, Vec<usize>, i64) {
        let simple = {
            let value_simple = find_value(header, "SIMPLE").unwrap_or(Value::Boolean(false));
            if let Value::Boolean(b) = value_simple {
                b
            } else {
                false
            }
        };

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

        (simple, naxis, axes, bitpix)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_from_bytes_test() {
            // Full keyword
            let keyword =
                "SIMPLE  =                    T / conforms to FITS standard                      ";
            let keyword_bytes = Vec::from_iter(keyword.bytes());
            let res = HeaderChunk::from_bytes(&keyword_bytes);
            assert_eq!(
                res.unwrap(),
                HeaderChunk::RawValue("SIMPLE", "T / conforms to FITS standard")
            );

            // No comment
            let keyword =
                "SIMPLE  =                    T                                                  ";
            let keyword_bytes = Vec::from_iter(keyword.bytes());
            let res = HeaderChunk::from_bytes(&keyword_bytes);
            assert_eq!(res.unwrap(), HeaderChunk::RawValue("SIMPLE", "T"));

            // No value separator
            let keyword =
                "COMMENT This is a comment, and therefore does not have a value separator.       ";
            let keyword_bytes = Vec::from_iter(keyword.bytes());
            let res = HeaderChunk::from_bytes(&keyword_bytes);
            assert_eq!(
                res.unwrap(),
                HeaderChunk::Comment(
                    "This is a comment, and therefore does not have a value separator."
                )
            );

            // End keyword
            let keyword =
                "END                                                                             ";
            let keyword_bytes = Vec::from_iter(keyword.bytes());
            let res = HeaderChunk::from_bytes(&keyword_bytes);
            assert_eq!(res.unwrap(), HeaderChunk::End);

            // Should fail:
            // Unexpected '/' in the string value
            let keyword =
                "KEYWORD =                       'something with a /       ' / and also a comment";
            let keyword_bytes = Vec::from_iter(keyword.bytes());
            let res = HeaderChunk::from_bytes(&keyword_bytes);
            assert_eq!(
                res.unwrap(),
                HeaderChunk::RawValue(
                    "KEYWORD",
                    "'something with a /       ' / and also a comment"
                )
            );

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

    pub fn chuncks_to_data_f64<'a>(
        blocks: &mut Chunks<'a, u8>,
        size: usize,
        bytes: u64,
    ) -> Vec<f64> {
        // NOTE: assume we are reading '64' floats:
        let mut data: Vec<f64> = Vec::with_capacity(size);
        let mut rem = bytes;
        while rem > 0 {
            let block = blocks.next().unwrap();
            let read = rem.min(definitions::BLOCK_SIZE as u64);
            for (x, _i) in block.chunks_exact((64 / 8) as usize).zip(0..read / 8) {
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

pub fn read_fits_buffer<'a>(buffer: &'a Vec<u8>) -> Option<(Header, Option<Tensor<f64>>)> {
    let mut blocks = buffer.chunks(definitions::BLOCK_SIZE);

    // Read header (PrimaryHDU) must always exist
    let header = header::parse_header(&mut blocks);
    let header = Header::from_keyword_list(header)?;
    // let (_simple, _naxis, axes, bitpix) = header::extract_values(&header);
    let bitpix = header.bitpix.to_int();
    let axes = &header.axes;

    // Calculate the total number of bytes
    let bytes: u64 = (axes.iter().product::<usize>() as u64 * (bitpix.abs() as u64)) / 8;
    // println!("Total bytes: {}", bytes);
    let size = axes.iter().product::<usize>();

    if bitpix == -64 {
        let data = data::chuncks_to_data_f64(&mut blocks, size, bytes);
        let data = Tensor::from(data);
        // Move the parsed data into the array
        // let arr = Array::from_vec(data);
        // let arr = arr.into_shape(axes).unwrap();

        // Print some random things
        // println!("{:?} {} {}", arr.shape(), arr.sum(), arr.mean().unwrap());
        return Some((header, Some(data)));
    } else {
        println!("Other data format; bitpix {}", bitpix);
        Some((header, None))
    }
}
