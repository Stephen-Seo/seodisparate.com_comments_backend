use std::num::ParseIntError;

use reqwest::StatusCode;
use salvo::{Depot, Request, Response, Writer, async_trait};

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
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
            Error::Reqwest(error) => error.fmt(f),
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

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Reqwest(value)
    }
}

#[async_trait]
impl Writer for Error {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Internal Server Error</b>
            </body></html>"#,
            crate::get_common_css(),
        ));
    }
}
