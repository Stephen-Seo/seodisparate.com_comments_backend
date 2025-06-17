// ISC License
//
// Copyright (c) 2025 Stephen Seo
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

use std::path::{Path, PathBuf};

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Args {
    config_file: PathBuf,
}

impl Args {
    pub fn parse_args() -> Result<Args, Error> {
        let mut args = std::env::args();

        args.next();

        let mut config_file: Option<PathBuf> = None;

        for arg in args {
            if arg == "-h" || arg == "--help" {
                println!("--config=<config_file>");
                return Err("-h | --help invoked!".into());
            } else if arg.starts_with("--config=") {
                let config_str = arg.clone().split_off(9);
                config_file = Some(config_str.into());
            }
        }

        Ok(Args {
            config_file: config_file.ok_or(Error::from("Config file not specified!"))?,
        })
    }

    pub fn get_config_path(&self) -> &Path {
        &self.config_file
    }
}
