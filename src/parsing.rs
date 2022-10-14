//! Parsing
//!
//! Contains all the code related to parsing header FITS cards, according to the
//! FITS standard. 
//! https://fits.gsfc.nasa.gov/standard40/fits_standard40aa-le.pdf
//!
//! General parsing workflow:
//! - Split into 80 character chuncks
//! - For each chunck
//!     - Determine type of card:
//!       Either commentary, or value.
//!       Keyword has a '= ' at positions 8,9. Commentary has optionally a
//!       '=' at 8 but not a ' ' at 9.

use std::{slice::ChunksExact, error::Error};

use crate::definitions;

#[derive(PartialEq, Debug)]
enum KeywordType {
    Commentary,
    Value,
    End,
}

impl KeywordType {
    fn from_raw(chunck: &[u8]) -> KeywordType {
        if chunck.starts_with(b"HISTORY ") || chunck.starts_with(b"COMMENT ") || chunck.starts_with(b"        ") {
            return KeywordType::Commentary
        }
        if chunck[8] == b'=' && chunck[9] == b' ' {
            return KeywordType::Value
        }
        return KeywordType::Commentary
    }

}

// Probably make this all 'String' or 'Vec<u8>' instead for ease.
pub enum Card<'a> {
    Commentary(&'a[u8], &'a[u8]),
    Value(&'a[u8], &'a[u8], &'a[u8]),
    Empty,
}

impl Card<'_> {
    fn parse_commentary<'a>(raw_card: &'a [u8]) -> Result<Card<'a>, dyn Error> {
        let (keyword, text) = raw_card.split_at(8);
        // remove the equal sign if it exists
        let text = if text[0] == b'=' {
            text.split_first().unwrap().1
        } else {
            text
        };
        Ok(Card::Commentary(keyword, text))
    }

    fn parse_value<'a>(raw_card: &'a [u8], _iter: &mut ChunksExact<u8> ) -> Card<'a> {
        
        // TODO: complete implementation, we need the `iter` since we can have
        // long strings.
        let (a, b) = raw_card.split_at(8);
        let (b, c) = b.split_at(2);
        Card::Value(a, b, c)
    }
}


pub fn parse_header<'a>(blocks: &'a mut ChunksExact<u8>) -> Vec<Card<'a>> {
    let mut end = false;
    let mut header = Vec::new();
    while !end {
        let block = blocks.next().expect("Incorrect FITS file: end of file while reading header. Expected 'END'");
        let mut cards_iter = block.chunks_exact(definitions::HEADER_KEYWORD_SIZE);

        while let Some(raw_card) = cards_iter.next() {
            let card = match KeywordType::from_raw(raw_card) {
                KeywordType::Commentary => {
                    Card::parse_commentary(raw_card)
                },
                KeywordType::Value => {
                    Card::parse_value(raw_card, &mut cards_iter)
                },
                KeywordType::End => {
                    // Stop completely
                    end = true;
                    break
                }
            };
            header.push(card);
        }

        let rem = cards_iter.remainder();
        if rem.len() > 0 {
            panic!("There was some remainder after the chuncks");
        }
    }
    return header;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commentary_test() {
        if let Card::Commentary(kw, v) = Card::parse_commentary(b"HISTORY =Someinterestingvalue") {
            assert_eq!(kw, b"HISTORY ");
            assert_eq!(v, b"Someinterestingvalue");
        } else{
            assert!(false);
        }

        if let Card::Commentary(kw, v) = Card::parse_commentary(b"COMMENT =Someotherinterestingvalue") {
            assert_eq!(kw, b"COMMENT ");
            assert_eq!(v, b"Someotherinterestingvalue");
        } else{
            assert!(false);
        }
    }

    #[test]
    fn keyword_type_test() {
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"COMMENT Someotherinterestingvalue"), "test 1");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"COMMENT = Someotherinterestingvalue"), "test 2");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"HISTORY Someotherinterestingvalue"), "test 3");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"HISTORY = Someotherinterestingvalue"), "test 4");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"        = Someotherinterestingvalue"), "test 5");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"        Someotherinterestingvalue"), "test 6");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"        =Someotherinterestingvalue"), "test 7");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"KEYWORD =Someotherinterestingvalue"), "test 8");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"KEYWORD Someotherinterestingvalue"), "test 9");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"KEYWORD = Someotherinterestingvalue"), "test 10");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"LONGKEYW= Someotherinterestingvalue"), "test 11");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"S       = Someotherinterestingvalue"), "test 12");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"L01--ABE= Someotherinterestingvalue"), "test 13");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"-_-_1AB9= Someotherinterestingvalue"), "test 14");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"-_-     = Someotherinterestingvalue"), "test 15");

        // These are not a valid keywords, they are intentionally not caught here
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b" KEYWORD= Someotherinterestingvalue"), "test 16");
        assert_eq!(KeywordType::Value, KeywordType::from_raw(b"A*EYWORD= Someotherinterestingvalue"), "test 17");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"********=Someotherinterestingvalue"), "test 18");
        assert_eq!(KeywordType::Commentary, KeywordType::from_raw(b"      **=Someotherinterestingvalue"), "test 19");
    }
}
