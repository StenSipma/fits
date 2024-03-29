// Link: https://fits.gsfc.nasa.gov/standard40/fits_standard40aa-le.pdf

#[allow(dead_code)]
mod definitions {
    pub const BLOCK_SIZE: usize = 2880; // bytes per block

    // Header sizes
    /// characters per header keyword
    pub const HEADER_KEYWORD_SIZE: usize = 80;
    /// the length (#chars) of the keyword name (i.e. NAXIS)
    pub const HEADER_KEYWORD_NAME_SIZE: usize = 8;
    /// the length (#chars) of the value indicator (i.e. '= ')
    pub const HEADER_VALUE_INDICATOR_SIZE: usize = 2;
    /// the length (#chars) of the value in a keyword
    pub const HEADER_VALUE_SIZE: usize = 70;

    pub const HEADER_VALUE_INDICATOR: &str = "= ";

    // Specific header keywords
    pub const HEADER_END_KEYWORD_FULL: &[u8] =
        b"END                                                                             ";
    pub const HEADER_END_KEYWORD: &str = "END";
    pub const HEADER_HISTORY_KEYWORD: &str = "HISTORY";
    pub const HEADER_COMMENT_KEYWORD: &str = "COMMENT";
    pub const HEADER_CONTINUE_KEYWORD: &str = "CONTINUE";

    // FITS with only a primary HDU is a 'Basic FITS File' or a 'Single Image FITS (SIF) File'
    // FITS with one or more extensions is a Multi-Extension FITS (MEF) file .
}

#[allow(dead_code)]
pub mod parsing;

use std::fs::File;
use std::io::Read;

use header::Header;
use ndarray::{Array, IxDyn};

type KeywordList = Vec<parsing::header::Keyword>;
type RawHeaderList<'a> = Vec<parsing::header::HeaderChunk<'a>>;
pub type GenericData<T> = Array<T, IxDyn>;

pub mod header {
    use tightness::bound;

    use crate::parsing::header::extract_values;
    use crate::KeywordList;

    pub struct Header {
        pub simple: bool,
        pub bitpix: Bitpix,
        pub naxis: Naxis,
        pub axes: Vec<usize>,
        pub keywords: KeywordList,
    }

    impl Header {
        pub fn from_keyword_list(keywords: KeywordList) -> Option<Self> {
            let (simple, naxis, axes, bitpix) = extract_values(&keywords)?;
            let naxis = Naxis::new(naxis).ok()?;
            let bitpix = Bitpix::from_int(bitpix)?;
            Some(Header {
                simple,
                bitpix,
                naxis,
                axes,
                keywords,
            })
        }

        pub fn print_keywords(&self) {
            for keyword in self.keywords.iter() {
                keyword.print()
            }
        }
    }

    // usize already guarentees that it is >= 0
    bound!(pub Naxis: usize where |u| (*u <= 999) );

    // See Table 8 of FITS standard (2018)
    #[derive(PartialEq, Debug, Clone)]
    pub enum Bitpix {
        Int8,    // 8
        Int16,   // 16
        Int32,   // 32
        Int64,   // 64
        Float32, // -32
        Float64, // -64
    }

    impl Bitpix {
        pub fn from_int(n: i64) -> Option<Self> {
            match n {
                8 => Some(Self::Int8),
                16 => Some(Self::Int16),
                32 => Some(Self::Int32),
                64 => Some(Self::Int64),
                -32 => Some(Self::Float32),
                -64 => Some(Self::Float64),
                _ => None,
            }
        }

        pub fn to_int(&self) -> i64 {
            match self {
                Self::Int8 => 8,
                Self::Int16 => 16,
                Self::Int32 => 32,
                Self::Int64 => 64,
                Self::Float32 => -32,
                Self::Float64 => -64,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn bitpix_test() {
            for i in vec![8, 16, 32, 64, -32, -64] {
                assert_eq!(i, Bitpix::from_int(i).unwrap().to_int())
            }

            assert!(Bitpix::from_int(0).is_none());
            assert!(Bitpix::from_int(-63).is_none());
            assert!(Bitpix::from_int(-8).is_none());
        }
    }
}

// Only basic FITS file for now, i.e. with one HDU
pub struct BasicFits {
    pub header: Header,
    pub data: GenericData<f64>,
}

impl BasicFits {
    pub fn from_bytes<'a>(bytes: Vec<u8>) -> Option<Self> {
        let (header, data) = parsing::read_fits_buffer(&bytes)?;
        let data = data.unwrap_or(GenericData::zeros(Vec::new()));
        let fits = BasicFits { header, data };
        Some(fits)
    }

    pub fn open<'a>(filename: &String) -> Option<Self> {
        let mut f = File::open(filename).ok()?;
        let mut buffer = Vec::new();

        if let Ok(_) = f.read_to_end(&mut buffer) {
            Self::from_bytes(buffer) // TODO: Check if this is good?
            // let (header, data) = parsing::read_fits_buffer(&buffer)?;
            // let data = data.unwrap_or(Tensor::new());
            // let fits = BasicFits { header, data };
            // Some(fits)
        } else {
            None
        }
    }
}
