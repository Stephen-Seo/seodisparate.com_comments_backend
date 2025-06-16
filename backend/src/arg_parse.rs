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
