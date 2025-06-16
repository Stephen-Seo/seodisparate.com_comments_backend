use std::num::ParseIntError;

#[derive(Debug)]
pub enum Error {
    ParseInt(ParseIntError),
    IO(std::io::Error),
    Mysql(mysql::Error),
    Generic(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(s) => f.write_str(&s),
            Error::IO(error) => error.fmt(f),
            Error::ParseInt(error) => error.fmt(f),
            Error::Mysql(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::Generic(value.to_owned())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Generic(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::ParseInt(value)
    }
}

impl From<mysql::Error> for Error {
    fn from(value: mysql::Error) -> Self {
        Error::Mysql(value)
    }
}
