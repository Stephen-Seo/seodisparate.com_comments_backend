// ISC License
//
// Copyright (c) 2025-2026 Stephen Seo
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
// REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
// AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
// INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
// LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
// OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
// PERFORMANCE OF THIS SOFTWARE.

use std::num::ParseIntError;

use reqwest::StatusCode;
use salvo::{Depot, Request, Response, Writer, async_trait};

#[derive(Debug)]
pub enum Error {
    StrUtf8(std::str::Utf8Error),
    TimeFormat(time::error::Format),
    TimeInvalFormat(time::error::InvalidFormatDescription),
    TimeIndetOffset(time::error::IndeterminateOffset),
    SerdeJson(serde_json::Error),
    SalvoHttpParse(salvo::http::ParseError),
    Reqwest(reqwest::Error),
    ParseInt(ParseIntError),
    IO(std::io::Error),
    Mysql(mysql::Error),
    Generic(String),
    ClientErr(Box<Error>),
}

impl Error {
    pub fn into_client_err(self) -> Self {
        Error::ClientErr(Box::new(self))
    }

    pub fn err_to_client_err<T>(error: T) -> Self
    where
        T: Into<Error>,
    {
        Error::ClientErr(Box::new(error.into()))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(s) => f.write_str(s),
            Error::IO(error) => error.fmt(f),
            Error::ParseInt(error) => error.fmt(f),
            Error::Mysql(error) => error.fmt(f),
            Error::Reqwest(error) => error.fmt(f),
            Error::SalvoHttpParse(error) => error.fmt(f),
            Error::ClientErr(error) => error.fmt(f),
            Error::SerdeJson(error) => error.fmt(f),
            Error::TimeIndetOffset(error) => error.fmt(f),
            Error::TimeInvalFormat(error) => error.fmt(f),
            Error::TimeFormat(error) => error.fmt(f),
            Error::StrUtf8(error) => error.fmt(f),
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

impl From<salvo::http::ParseError> for Error {
    fn from(value: salvo::http::ParseError) -> Self {
        Error::SalvoHttpParse(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::SerdeJson(value)
    }
}

impl From<time::error::IndeterminateOffset> for Error {
    fn from(value: time::error::IndeterminateOffset) -> Self {
        Error::TimeIndetOffset(value)
    }
}

impl From<time::error::InvalidFormatDescription> for Error {
    fn from(value: time::error::InvalidFormatDescription) -> Self {
        Error::TimeInvalFormat(value)
    }
}

impl From<time::error::Format> for Error {
    fn from(value: time::error::Format) -> Self {
        Error::TimeFormat(value)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Error::StrUtf8(value)
    }
}

#[async_trait]
impl Writer for Error {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        eprintln!("{:?}", &self);
        match &self {
            Error::ClientErr(_error) => {
                res.status_code(StatusCode::BAD_REQUEST);
                res.body(format!(
                    r#"<html><head><style>{}</style></head><body>
                    <b>Bad Request</b>
                    </body></html>"#,
                    crate::COMMON_CSS,
                ));
            }
            _ => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.body(format!(
                    r#"<html><head><style>{}</style></head><body>
                    <b>Internal Server Error</b>
                    </body></html>"#,
                    crate::COMMON_CSS,
                ));
            }
        }
    }
}
