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

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct Config {
    sql_user: String,
    sql_pass: String,
    sql_addr: String,
    sql_port: String,
    sql_db: String,
    tcp_addr: String,
    tcp_port: u16,
    oauth_user: String,
    oauth_token: String,
    base_url: String,
    allowed_urls: Vec<String>,
    allowed_bids: Vec<String>,
    user_agent: String,
    on_comment_cmds: Vec<String>,
    admins: Vec<String>,
}

impl Config {
    pub fn get_connection_string(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.sql_user, self.sql_pass, self.sql_addr, self.sql_port, self.sql_db
        )
    }

    pub fn get_sql_db(&self) -> &str {
        &self.sql_db
    }

    pub fn get_addr(&self) -> &str {
        &self.tcp_addr
    }

    pub fn get_port(&self) -> u16 {
        self.tcp_port
    }

    pub fn get_oauth_user(&self) -> &str {
        &self.oauth_user
    }

    pub fn get_oauth_token(&self) -> &str {
        &self.oauth_token
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get_allowed_urls(&self) -> &[String] {
        &self.allowed_urls
    }

    pub fn get_allowed_bids(&self) -> &[String] {
        &self.allowed_bids
    }

    pub fn get_user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn get_on_comment_cmds(&self) -> &[String] {
        &self.on_comment_cmds
    }

    pub fn get_admins(&self) -> &[String] {
        &self.admins
    }
}

impl TryFrom<&Path> for Config {
    type Error = crate::error::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let file = File::open(value)?;
        let file_buffered = BufReader::new(file);

        let mut sql_user: Result<String, Self::Error> = Err("sql_user not specified!".into());
        let mut sql_pass: Result<String, Self::Error> = Err("sql_pass not specified!".into());
        let mut sql_addr: Result<String, Self::Error> = Err("sql_addr not specified!".into());
        let mut sql_port: Result<String, Self::Error> = Err("sql_port not specified!".into());
        let mut sql_db: Result<String, Self::Error> = Err("sql_db not specified!".into());
        let mut tcp_addr: String = "127.0.0.1".into();
        let mut tcp_port: u16 = 8000;
        let mut oauth_user: Result<String, Self::Error> = Err("oauth_user not specified!".into());
        let mut oauth_token: Result<String, Self::Error> = Err("oauth_token not specified!".into());
        let mut base_url: Result<String, Self::Error> = Err("base_url not specified!".into());
        let mut allowed_urls: Vec<String> = Vec::new();
        let mut allowed_bids: Vec<String> = Vec::new();
        let mut user_agent: Result<String, Self::Error> = Err("user_agent not specified!".into());

        let mut on_comment_cmds: Vec<String> = Vec::new();

        let mut admins: Vec<String> = Vec::new();

        let mut key: String = String::new();
        let mut val: String = String::new();
        let mut is_parsing_key = true;
        for byte in file_buffered.bytes() {
            let c: char = byte?.into();
            if c == '\r' {
                continue;
            }
            if is_parsing_key {
                if c == '=' {
                    is_parsing_key = false;
                } else {
                    key.push(c);
                }
            } else if c == '\n' {
                is_parsing_key = true;
                if key == "sql_user" {
                    sql_user = Ok(val);
                } else if key == "sql_pass" {
                    sql_pass = Ok(val);
                } else if key == "sql_addr" {
                    sql_addr = Ok(val);
                } else if key == "sql_port" {
                    sql_port = Ok(val);
                } else if key == "sql_db" {
                    sql_db = Ok(val);
                } else if key == "tcp_addr" {
                    tcp_addr = val;
                } else if key == "tcp_port" {
                    tcp_port = val.parse()?;
                } else if key == "oauth_user" {
                    oauth_user = Ok(val);
                } else if key == "oauth_token" {
                    oauth_token = Ok(val);
                } else if key == "base_url" {
                    base_url = Ok(val);
                } else if key == "allowed_url" {
                    allowed_urls.push(val);
                } else if key == "allowed_bid" {
                    allowed_bids.push(val);
                } else if key == "user_agent" {
                    user_agent = Ok(val);
                } else if key == "on_comment_cmd" {
                    on_comment_cmds.push(val);
                } else if key == "admin" {
                    admins.push(val);
                } else {
                    println!("WARNING: Got unknown config key \"{}\"!", key);
                }
                key = String::new();
                val = String::new();
            } else {
                val.push(c);
            }
        }

        if !key.is_empty() && !val.is_empty() {
            if key == "sql_user" {
                sql_user = Ok(val);
            } else if key == "sql_pass" {
                sql_pass = Ok(val);
            } else if key == "sql_addr" {
                sql_addr = Ok(val);
            } else if key == "sql_port" {
                sql_port = Ok(val);
            } else if key == "sql_db" {
                sql_db = Ok(val);
            } else if key == "tcp_addr" {
                tcp_addr = val;
            } else if key == "tcp_port" {
                tcp_port = val.parse()?;
            } else if key == "oauth_user" {
                oauth_user = Ok(val);
            } else if key == "oauth_token" {
                oauth_token = Ok(val);
            } else if key == "base_url" {
                base_url = Ok(val);
            } else if key == "allowed_url" {
                allowed_urls.push(val);
            } else if key == "allowed_bid" {
                allowed_bids.push(val);
            } else if key == "user_agent" {
                user_agent = Ok(val);
            } else if key == "on_comment_cmd" {
                on_comment_cmds.push(val);
            } else if key == "admin" {
                admins.push(val);
            } else {
                println!("WARNING: Got unknown config key \"{}\"!", key);
            }
        }

        Ok(Config {
            sql_user: sql_user?,
            sql_pass: sql_pass?,
            sql_addr: sql_addr?,
            sql_port: sql_port?,
            sql_db: sql_db?,
            tcp_addr,
            tcp_port,
            oauth_user: oauth_user?,
            oauth_token: oauth_token?,
            base_url: base_url?,
            allowed_urls,
            allowed_bids,
            user_agent: user_agent?,
            on_comment_cmds,
            admins,
        })
    }
}
