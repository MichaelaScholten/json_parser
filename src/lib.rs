#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{iter::Peekable, str::FromStr};

use itertools::{Itertools as _, PeekingNext};

/// An error occured while trying to parse the json file
#[derive(Debug)]
pub enum Error {
    /// An invalid character in a JSON file was found
    InvalidValue,

    /// A string wasn't closed
    UnclosedString,

    /// A list/array wasn't closed
    UnclosedList,

    /// A value separator (',' or ':') is missing
    MissingSeparator,

    /// The byte stream ended unexpectedly
    UnexpectedEndOfFile,

    /// An object wasn't closed
    UnclosedObject,
}

/// A JSON value
#[derive(Debug, PartialEq)]
pub enum Json {
    /// A list of data
    List(Vec<Json>),

    /// An object
    Object(Vec<(String, Json)>),

    /// A string
    String(String),

    /// A number
    Number(f64),

    /// A boolean
    Bool(bool),

    /// A null value
    Null,
}

impl Json {
    /// Tries to read a string value
    fn read_string<I: PeekingNext<Item = char>>(mut iter: I) -> Result<String, Error> {
        // Make sure the value started with "
        if iter.next() != Some('"') {
            return Err(Error::InvalidValue);
        }

        // Read the string
        let mut escaped = false;
        let result = iter
            .peeking_take_while(|&c| {
                let keep_reading = escaped || c != '"';
                escaped = !escaped && c == '\\';
                keep_reading
            })
            .collect();

        // Make sure the string actually ended
        if iter.next() != Some('"') || escaped {
            return Err(Error::UnclosedString);
        }

        Ok(result)
    }

    /// Tries to read a boolean
    fn read_bool<I: Iterator<Item = char>>(mut iter: I) -> Result<bool, Error> {
        // Read the first character of the boolean
        match iter.next() {
            // If it's an f, make sure the value is false.
            // Return an error otherwise.
            Some('f') => {
                if iter
                    .zip("alse".chars())
                    .filter(|(found, expected)| found == expected)
                    .take(4)
                    .count()
                    == 4
                {
                    Ok(false)
                } else {
                    Err(Error::InvalidValue)
                }
            }

            // If the first character is a t, make sure the value is true.
            // Return an error otherwise.
            Some('t') => {
                if iter
                    .zip("rue".chars())
                    .filter(|(found, expected)| found == expected)
                    .take(3)
                    .count()
                    == 3
                {
                    Ok(true)
                } else {
                    Err(Error::InvalidValue)
                }
            }

            // Return an error if the value isn't a boolean
            None | Some(_) => Err(Error::InvalidValue),
        }
    }

    /// Tries to read a null value
    fn read_null<I: Iterator<Item = char>>(iter: I) -> Result<(), Error> {
        // Make sure the value is null, return an error otherwise.
        if iter
            .zip("null".chars())
            .filter(|(found, expected)| found == expected)
            .take(4)
            .count()
            == 4
        {
            Ok(())
        } else {
            Err(Error::InvalidValue)
        }
    }

    /// Tries to read a numeric value
    fn read_number<I: PeekingNext<Item = char>>(mut iter: I) -> Result<f64, Error> {
        // Read the characters of the number into a string
        let result = iter
            .peeking_take_while(|&ch| matches!(ch, '0'..='9' | '.' | '+' | '-'))
            .collect::<String>();

        // Return an error if the string is empty
        if result.is_empty() {
            return Err(Error::InvalidValue);
        }

        // Try to parse an error, return an error on failure
        match result.parse::<f64>() {
            Err(_) => Err(Error::InvalidValue),
            Ok(number) => Ok(number),
        }
    }

    /// Skips whitespace without wasting characters
    fn skip_whitespace<I: PeekingNext<Item = char>>(mut iter: I) {
        iter.peeking_take_while(|&ch| ch.is_whitespace())
            .for_each(|_| {});
    }

    /// Tries to parse a json value
    fn parse_value<I: Iterator<Item = char>>(mut iter: &mut Peekable<I>) -> Result<Self, Error> {
        Ok(
            // Read the first character
            match iter.peek() {
                // If it's a ", try to read and return the string
                Some('"') => Self::String(Self::read_string(&mut iter)?),

                // If it's a t or an f, try to read and the bool
                Some('t' | 'f') => Self::Bool(Json::read_bool(&mut iter)?),

                // If it's an n, make sure it's null and return it
                Some('n') => {
                    Self::read_null(&mut iter)?;
                    Self::Null
                }

                // If it's numeric, try to parse and return the number
                Some('0'..='9' | '.' | '-' | '+') => Self::Number(Self::read_number(&mut iter)?),

                // If it's [, try to parse and return the list
                Some('[') => Self::List(Self::read_list(iter)?),

                // If it's {, try to parse and return the object
                Some('{') => Self::Object(Self::read_object(iter)?),

                // If it is a different value, return it
                Some(_) => return Err(Error::InvalidValue),

                // If there is no value, return an error
                None => return Err(Error::UnexpectedEndOfFile),
            },
        )
    }

    /// Tries to parse a list of data
    fn read_list<I: Iterator<Item = char>>(mut iter: &mut Peekable<I>) -> Result<Vec<Self>, Error> {
        // Make sure the first character is a [
        if iter.next() != Some('[') {
            return Err(Error::InvalidValue);
        }

        // Read the list
        let mut result = Vec::new();
        loop {
            // Find the value or closing character
            Self::skip_whitespace(&mut iter);

            // Stop if the closing character has been found
            if iter.peek() == Some(&']') {
                iter.next().unwrap();
                break;
            }

            // Add the value to the list
            result.push(Self::parse_value(iter)?);

            // Find the seperator or closing character
            match iter.find(|&ch| !ch.is_whitespace()) {
                // Stop if the closing character has been found
                Some(']') => break,

                // Skip the value separator
                Some(',') => {}

                // Return an error if neither was found
                Some(_) => return Err(Error::MissingSeparator),

                // Return an error if there are no chars left
                None => return Err(Error::UnclosedList),
            }
        }
        Ok(result)
    }

    /// Tries to read an object
    fn read_object<I: Iterator<Item = char>>(
        mut iter: &mut Peekable<I>,
    ) -> Result<Vec<(String, Self)>, Error> {
        // Return an error if the object isn't an object
        if iter.next() != Some('{') {
            return Err(Error::InvalidValue);
        }

        // Read the object
        let mut result = Vec::new();
        loop {
            // Skip whitespace
            Self::skip_whitespace(&mut iter);

            // Stop if the end of the object has been found
            if iter.peek() == Some(&'}') {
                iter.next().unwrap();
                break;
            }

            // Read the name of the property
            let name = Self::read_string(&mut iter)?;

            // Skip whitespace
            Self::skip_whitespace(&mut iter);

            // Make sure the key-value separator was found
            if iter.next() != Some(':') {
                return Err(Error::MissingSeparator);
            }

            // Skip whitespace
            Self::skip_whitespace(&mut iter);

            // Try to parse the found value
            let value = Self::parse_value(iter)?;

            // Insert the property with name and value
            result.push((name, value));

            // Skip the whitespace
            Self::skip_whitespace(&mut iter);

            // Check the next character
            match iter.next() {
                // Stop if the end of the object has been found
                Some('}') => break,

                // Skip the value separator
                Some(',') => {}

                // Return an error if an other character was found
                Some(_) => return Err(Error::MissingSeparator),

                // Return an error if there are no chars left
                None => return Err(Error::UnclosedObject),
            }
        }
        Ok(result)
    }

    /// Parses a JSON value from characters
    pub fn from_chars<I: Iterator<Item = char>>(iter: I) -> Result<Self, Error> {
        Self::parse_value(&mut iter.skip_while(|ch| ch.is_whitespace()).peekable())
    }

    /// Parses a JSON value from bytes (if the byte to char conversion works well enough)
    pub fn from_bytes<I: Iterator<Item = u8>>(iter: I) -> Result<Self, Error> {
        Self::from_chars(Chars(iter))
    }
}

impl FromStr for Json {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_chars(s.chars())
    }
}

/// Converts the items from an iterator to characters
struct Chars<I>(I);

impl<I: Iterator<Item = u8>> Iterator for Chars<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        // Convert a series of bytes to a single char
        let mut value = 0;
        for byte in self.0.by_ref().take(4) {
            // Add the byte to the number
            value = (value << 8) | u32::from(byte);

            // Try to convert the number to a char and return it on success
            if let Some(ch) = char::from_u32(value) {
                return Some(ch);
            }
        }

        // Return None on failure
        None
    }
}

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned, vec::Vec};

    use crate::Json;

    #[test]
    fn string_parsing() {
        assert_eq!(Json::read_string("\"\"".chars()).unwrap(), "");
        assert!(Json::read_string("".chars()).is_err());
        assert!(Json::read_string("\"".chars()).is_err());
    }

    #[test]
    fn bool_parsing() {
        assert!(Json::read_bool("true".chars()).unwrap());
        assert!(Json::read_bool("tru".chars()).is_err());
        assert!(!Json::read_bool("false".chars()).unwrap());
        assert!(Json::read_bool("fals".chars()).is_err());
    }

    #[test]
    fn null_parsing() {
        Json::read_null("null".chars()).unwrap();
        assert!(Json::read_null("nu".chars()).is_err());
    }

    #[test]
    fn number_parsing() {
        assert_eq!(Json::read_number("-123.456".chars()).unwrap(), -123.456);
        assert!(Json::read_number("hello".chars()).is_err());
    }

    #[test]
    fn list_parsing() {
        assert!(Json::read_list(&mut "{}".chars().peekable()).is_err());
        assert_eq!(
            Json::read_list(&mut "[]".chars().peekable()).unwrap(),
            Vec::new()
        );
        assert_eq!(
            Json::read_list(&mut "[-654.321, {},[], \"Hello\",false,null]".chars().peekable())
                .unwrap(),
            [
                Json::Number(-654.321),
                Json::Object(Vec::new()),
                Json::List(Vec::new()),
                Json::String("Hello".to_owned()),
                Json::Bool(false),
                Json::Null
            ]
        );
    }

    #[test]
    fn object_parsing() {
        assert!(Json::read_object(&mut "[]".chars().peekable()).is_err());
        assert_eq!(
            Json::read_object(&mut "{}".chars().peekable()).unwrap(),
            Vec::new()
        );
        assert_eq!(
            Json::read_object(&mut "{\"number\":-123.456,\"object\":{}}".chars().peekable())
                .unwrap(),
            Vec::from([
                ("number".to_owned(), Json::Number(-123.456)),
                ("object".to_owned(), Json::Object(Vec::new()))
            ])
        );
        assert_eq!(
            Json::read_object(
                &mut "{\"number\":-123.456,\"object\":{},\"list\":[],\"string\": \"Hello\", \"bool\": true ,\"null\":null}".chars().peekable()
            ).unwrap(),
            Vec::from([
                ("number".to_owned(), Json::Number(-123.456)),
                ("object".to_owned(), Json::Object(Vec::new())),
                ("list".to_owned(), Json::List(Vec::new())),
                ("string".to_owned(), Json::String("Hello".to_owned())),
                ("bool".to_owned(), Json::Bool(true)),
                ("null".to_owned(), Json::Null)
            ])
        );
    }
}
