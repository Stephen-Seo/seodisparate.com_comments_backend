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

use std::sync::{Arc, Mutex};

use crate::{Config, error::Error};
use msql_ffi::{MSQLParamsWrapper, MSQLWrapper};
use serde::Serialize;
use time::{PrimitiveDateTime, UtcOffset, format_description};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Comment {
    pub comment_id: String,
    pub username: String,
    pub userurl: String,
    pub useravatar: String,
    pub create_date: String,
    pub edit_date: String,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoComment {
    pub user_id: u64,
    pub username: String,
    pub userurl: String,
    pub useravatar: String,
    pub blog_post_id: String,
    pub comment_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginInfo {
    pub user_id: String,
    pub user_ip: Option<String>,
    pub user_github_id: u64,
    pub username: String,
    pub userlogin: String,
    pub userurl: String,
    pub useravatar: String,
}

#[derive(Clone)]
pub enum SQLCtx {
    Connection(Arc<Mutex<MSQLWrapper>>),
    Config {
        user: String,
        pass: String,
        addr: String,
        port: u16,
        db: String,
    },
}

impl SQLCtx {
    pub fn new_as_connection(config: &Config) -> Result<Self, Error> {
        MSQLWrapper::try_new(
            config.get_sql_addr(),
            config.get_sql_port(),
            config.get_sql_user(),
            config.get_sql_pass(),
            config.get_sql_db(),
        )
        .map_err(|_| -> Error { "Failed to create msql connection".into() })
        .map(|w| SQLCtx::Connection(Arc::new(Mutex::new(w))))
    }
}

impl From<&Config> for SQLCtx {
    fn from(value: &Config) -> Self {
        SQLCtx::Config {
            user: value.get_sql_user().to_owned(),
            pass: value.get_sql_pass().to_owned(),
            addr: value.get_sql_addr().to_owned(),
            port: value.get_sql_port(),
            db: value.get_sql_db().to_owned(),
        }
    }
}

impl From<Arc<Mutex<MSQLWrapper>>> for SQLCtx {
    fn from(value: Arc<Mutex<MSQLWrapper>>) -> Self {
        SQLCtx::Connection(value)
    }
}

impl TryFrom<SQLCtx> for Arc<Mutex<MSQLWrapper>> {
    type Error = crate::Error;

    fn try_from(value: SQLCtx) -> Result<Self, Self::Error> {
        match value {
            SQLCtx::Connection(msqlwrapper) => Ok(msqlwrapper),
            SQLCtx::Config {
                user,
                pass,
                addr,
                port,
                db,
            } => MSQLWrapper::try_new(&addr, port, &user, &pass, &db)
                .map(|w| Arc::new(Mutex::new(w)))
                .map_err(|_| -> Error { "Failed to create msql connection from Config".into() }),
        }
    }
}

pub fn set_up_sql_db(sql_ctx: SQLCtx, config: &Config) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS COMMENT2 (
            uuid CHAR(36) PRIMARY KEY,
            state CHAR(36),
            INDEX state_index USING HASH (state),
            blog_post_id TINYTEXT,
            INDEX blog_post_id_index USING HASH (blog_post_id),
            user_id BIGINT,
            INDEX user_id_index USING HASH (user_id),
            username TINYTEXT,
            userurl TINYTEXT,
            useravatar TINYTEXT,
            creation_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            INDEX creation_date_index USING BTREE (creation_date),
            edit_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            timeout_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            comment TEXT
        )",
    )?;

    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS LOGIN (
            id CHAR(36) PRIMARY KEY,
            ip TINYTEXT,
            INDEX ip_index USING HASH (ip),
            user_id BIGINT NOT NULL,
            username TINYTEXT NOT NULL,
            userlogin TINYTEXT NOT NULL,
            userurl TINYTEXT NOT NULL,
            useravatar TINYTEXT NOT NULL,
            login_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )?;

    {
        let mut params: MSQLParamsWrapper = MSQLParamsWrapper::new();
        params.append_str(config.get_sql_db())?;
        let rows_res = conn.query_with_params_rows("SELECT * FROM information_schema.tables WHERE table_schema = ? AND table_name = 'COMMENT'", &params).map_err(|e| Error::Generic(e.to_owned()))?;

        if rows_res.is_some() {
            // Migrate from COMMENT to COMMENT2 if exists.
            conn.query_drop(
                r"INSERT INTO COMMENT2 (uuid, blog_post_id, user_id, username, userurl, useravatar, creation_date, edit_date, comment, timeout_date)
                    SELECT uuid, blog_post_id, user_id, username, userurl, useravatar, creation_date, edit_date, comment, NULL FROM COMMENT
                "
            )?;

            conn.query_drop(r"DROP TABLE COMMENT")?;
        }
    }

    // Drop unused tables. The data in these tables were meant to be temporary
    // so no migration is required for them.
    conn.query_drop(r"DROP TABLE IF EXISTS PSEUDO_COMMENT")?;
    conn.query_drop(r"DROP TABLE IF EXISTS GITHUB_RNG")?;

    Ok(())
}

pub fn has_psuedo_commment_with_state(sql_ctx: SQLCtx, state: &str) -> Result<bool, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(state)?;

    let rows = conn
        .query_with_params_rows("SELECT uuid FROM COMMENT2 WHERE uuid = ?", &params)
        .map_err(|e| Error::Generic(e.to_owned()))?;

    Ok(rows.is_some())
}

pub fn create_rng_uuid(sql_ctx: SQLCtx, uuid: Option<&str>) -> Result<String, Error> {
    // Ensure the sql_ctx has a connection.
    let sql_ctx: SQLCtx = TryInto::<Arc<Mutex<MSQLWrapper>>>::try_into(sql_ctx)?.into();

    {
        let sql_ctx_clone = sql_ctx.clone();
        let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx_clone.try_into()?;
        let mut conn = conn
            .try_lock()
            .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

        conn.query_drop(
            r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
        )?;
    }

    let mut rng_uuid = uuid::Uuid::new_v4();

    loop {
        let sql_ctx_clone = sql_ctx.clone();

        let ret: bool = has_psuedo_commment_with_state(sql_ctx_clone, &rng_uuid.to_string())?;

        if !ret {
            break;
        }

        rng_uuid = uuid::Uuid::new_v4();
    }

    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let rng_uuid_string = rng_uuid.to_string();

    if let Some(uuid_str) = uuid {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(&rng_uuid_string)?;
        params.append_str(uuid_str)?;

        conn.query_with_params_drop(
            "UPDATE COMMENT2 SET state = ? WHERE uuid = ? AND timeout_date IS NULL",
            &params,
        )?;

        params = MSQLParamsWrapper::new();
        params.append_str(uuid_str)?;

        let rows = conn.query_with_params_rows(
            "SELECT state FROM COMMENT2 WHERE state IS NOT NULL AND uuid = ?",
            &params,
        )?;

        if rows.is_none() {
            return Err("Failed to add state to existing comment!".into());
        }
    } else {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(&rng_uuid_string)?;

        conn.query_with_params_drop("INSERT INTO COMMENT2 (uuid) VALUES (?)", &params)?;
    }

    Ok(rng_uuid_string)
}

pub fn check_rng_uuid(sql_ctx: SQLCtx, uuid: &str, state: Option<&str>) -> Result<bool, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    if let Some(state) = state {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(uuid)?;
        params.append_str(state)?;

        let rows = conn.query_with_params_rows(
            "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND state = ? AND timeout_date IS NULL",
            &params,
        )?;

        Ok(rows.is_some())
    } else {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(uuid)?;

        let rows = conn.query_with_params_rows(
            "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NOT NULL",
            &params,
        )?;

        Ok(rows.is_some())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn add_pseudo_comment_data(
    sql_ctx: SQLCtx,
    state: &str,
    user_id: u64,
    user_name: &str,
    user_url: &str,
    user_avatar_url: &str,
    blog_post_id: Option<&str>,
    comment_id: Option<&str>,
) -> Result<String, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    if let Some(blog_id) = blog_post_id {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(state)?;

        let rows = conn.query_with_params_rows(
            "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NOT NULL",
            &params,
        )?;

        if rows.is_none() {
            return Err("Timed out creating comment!".into());
        }

        params = MSQLParamsWrapper::new();
        params.append_uint64(user_id);
        params.append_str(user_name)?;
        params.append_str(user_url)?;
        params.append_str(user_avatar_url)?;
        params.append_str(blog_id)?;
        params.append_str(state)?;

        conn.query_with_params_drop("UPDATE COMMENT2 SET user_id=?, username=?, userurl=?, useravatar=?, blog_post_id=? WHERE uuid = ?", &params)?;
    } else if let Some(comment_id) = comment_id {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(comment_id)?;
        params.append_str(state)?;

        let rows = conn.query_with_params_rows(
            "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND state = ? AND timeout_date IS NULL",
            &params,
        )?;

        if rows.is_none() {
            return Err("Timed out creating comment!".into());
        }

        params = MSQLParamsWrapper::new();
        params.append_uint64(user_id);
        params.append_str(user_name)?;
        params.append_str(user_url)?;
        params.append_str(user_avatar_url)?;
        params.append_str(comment_id)?;

        conn.query_with_params_drop("UPDATE COMMENT2 SET user_id=?, username=?, userurl=?, useravatar=?, state=NULL WHERE uuid = ?", &params)?;
    }

    Ok(state.to_string())
}

pub fn add_comment(sql_ctx: SQLCtx, state: &str, comment: &str) -> Result<PseudoComment, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(state)?;

    let rows = conn.query_with_params_rows("SELECT uuid FROM COMMENT2 WHERE uuid = ?", &params)?;

    if rows.is_none() {
        return Err("Timed out creating comment!".into());
    }

    params = MSQLParamsWrapper::new();
    params.append_str(comment)?;
    params.append_str(state)?;

    conn.query_with_params_drop(
        "UPDATE COMMENT2 SET timeout_date=NULL, comment=? WHERE uuid = ?",
        &params,
    )?;

    params = MSQLParamsWrapper::new();
    params.append_str(state)?;

    let rows = conn.query_with_params_rows(
        "SELECT user_id, username, userurl, useravatar, blog_post_id FROM COMMENT2 WHERE uuid = ?",
        &params,
    )?;

    let user_id: Option<u64>;
    let username: Option<String>;
    let userurl: Option<String>;
    let useravatar: Option<String>;
    let blog_post_id: Option<String>;

    if let Some(rows) = rows {
        if rows.len() == 1 && rows[0].len() == 5 {
            user_id = match &rows[0][0] {
                msql_ffi::MSQLValueEnum::Error => None,
                msql_ffi::MSQLValueEnum::Null => None,
                msql_ffi::MSQLValueEnum::Int64(i) => Some(*i as u64),
                msql_ffi::MSQLValueEnum::UInt64(u) => Some(*u),
                msql_ffi::MSQLValueEnum::String(_) => None,
                msql_ffi::MSQLValueEnum::DoubleF64(_) => None,
            };

            username = match &rows[0][1] {
                msql_ffi::MSQLValueEnum::Error => None,
                msql_ffi::MSQLValueEnum::Null => None,
                msql_ffi::MSQLValueEnum::Int64(_) => None,
                msql_ffi::MSQLValueEnum::UInt64(_) => None,
                msql_ffi::MSQLValueEnum::String(s) => Some(s.to_owned()),
                msql_ffi::MSQLValueEnum::DoubleF64(_) => None,
            };

            userurl = match &rows[0][2] {
                msql_ffi::MSQLValueEnum::Error => None,
                msql_ffi::MSQLValueEnum::Null => None,
                msql_ffi::MSQLValueEnum::Int64(_) => None,
                msql_ffi::MSQLValueEnum::UInt64(_) => None,
                msql_ffi::MSQLValueEnum::String(s) => Some(s.to_owned()),
                msql_ffi::MSQLValueEnum::DoubleF64(_) => None,
            };

            useravatar = match &rows[0][3] {
                msql_ffi::MSQLValueEnum::Error => None,
                msql_ffi::MSQLValueEnum::Null => None,
                msql_ffi::MSQLValueEnum::Int64(_) => None,
                msql_ffi::MSQLValueEnum::UInt64(_) => None,
                msql_ffi::MSQLValueEnum::String(s) => Some(s.to_owned()),
                msql_ffi::MSQLValueEnum::DoubleF64(_) => None,
            };

            blog_post_id = match &rows[0][4] {
                msql_ffi::MSQLValueEnum::Error => None,
                msql_ffi::MSQLValueEnum::Null => None,
                msql_ffi::MSQLValueEnum::Int64(_) => None,
                msql_ffi::MSQLValueEnum::UInt64(_) => None,
                msql_ffi::MSQLValueEnum::String(s) => Some(s.to_owned()),
                msql_ffi::MSQLValueEnum::DoubleF64(_) => None,
            };
        } else {
            return Err("Add comment: Failed to query pseudo comment (invalid length)".into());
        }
    } else {
        return Err("Add comment: Failed to query pseudo comment (does not exist)".into());
    }

    Ok(PseudoComment {
        user_id: user_id.ok_or(Into::<Error>::into("Add comment: Failed to parse user_id"))?,
        username: username.ok_or(Into::<Error>::into("Add comment: Failed to parse username"))?,
        userurl: userurl.ok_or(Into::<Error>::into("Add comment: Failed to parse userurl"))?,
        useravatar: useravatar.ok_or(Into::<Error>::into(
            "Add comment: Failed to parse useravatar",
        ))?,
        blog_post_id: blog_post_id.ok_or(Into::<Error>::into(
            "Add comment: Failed to parse blog_post_id",
        ))?,
        comment_id: String::new(),
    })
}

pub fn check_edit_comment_auth(sql_ctx: SQLCtx, cid: &str, uid: &str) -> Result<bool, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(cid)?;
    params.append_str(uid)?;

    let rows = conn.query_with_params_rows(
        "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND user_id = ?",
        &params,
    )?;

    Ok(rows.is_some())
}

pub fn get_comment_text(sql_ctx: SQLCtx, cid: &str) -> Result<String, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(cid)?;

    let rows =
        conn.query_with_params_rows("SELECT comment from COMMENT2 WHERE uuid = ?", &params)?;

    if let Some(rows) = rows
        && rows.len() == 1
        && rows[0].len() == 1
    {
        match &rows[0][0] {
            msql_ffi::MSQLValueEnum::Error => Err("Internal error fetching comment".into()),
            msql_ffi::MSQLValueEnum::Null => Err("Internal error fetching comment".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => Err("Internal error fetching comment".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => Err("Internal error fetching comment".into()),
            msql_ffi::MSQLValueEnum::String(s) => Ok(s.to_owned()),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => Err("Internal error fetching comment".into()),
        }
    } else {
        Err("Internal error querying comment".into())
    }
}

pub fn edit_comment(sql_ctx: SQLCtx, uuid: &str, comment: &str) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(comment)?;
    params.append_str(uuid)?;

    conn.query_with_params_drop(
        "UPDATE COMMENT2 SET edit_date = CURRENT_TIMESTAMP, comment = ? WHERE uuid = ?",
        &params,
    )?;

    Ok(())
}

pub fn try_delete_comment(sql_ctx: SQLCtx, cid: &str, uid: u64) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(cid)?;
    params.append_uint64(uid);

    conn.query_with_params_drop(
        "DELETE FROM COMMENT2 WHERE uuid = ? AND user_id = ?",
        &params,
    )?;

    Ok(())
}

pub fn try_delete_comment_id_only(sql_ctx: SQLCtx, cid: &str) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(cid)?;

    conn.query_with_params_drop("DELETE FROM COMMENT2 WHERE uuid = ?", &params)?;

    Ok(())
}

pub fn get_comments_per_blog_id(sql_ctx: SQLCtx, blog_id: &str) -> Result<Vec<Comment>, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let utc_offset = UtcOffset::current_local_offset()?;

    let parsing_format =
        format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")?;

    let output_format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
    )?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(blog_id)?;

    let rows = conn.query_with_params_rows("SELECT uuid, username, userurl, useravatar, creation_date, edit_date, comment FROM COMMENT2 WHERE blog_post_id = ? ORDER BY creation_date", &params)?;

    if rows.is_none() {
        // No comments.
        return Ok(Vec::new());
    }

    let mut comments: Vec<Comment> = Vec::new();

    for row in rows.as_ref().unwrap() {
        let comment_id: String = match &row[0] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let username: String = match &row[1] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let userurl: String = match &row[2] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let useravatar: String = match &row[3] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let create_date: PrimitiveDateTime = match &row[4] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => {
                let res = PrimitiveDateTime::parse(s, &parsing_format);

                if let Ok(ret_time) = res {
                    ret_time
                } else {
                    continue;
                }
            }
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let edit_date: PrimitiveDateTime = match &row[5] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => {
                let res = PrimitiveDateTime::parse(s, &parsing_format);

                if let Ok(ret_time) = res {
                    ret_time
                } else {
                    continue;
                }
            }
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };
        let comment: String = match &row[6] {
            msql_ffi::MSQLValueEnum::Error => continue,
            msql_ffi::MSQLValueEnum::Null => continue,
            msql_ffi::MSQLValueEnum::Int64(_) => continue,
            msql_ffi::MSQLValueEnum::UInt64(_) => continue,
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => continue,
        };

        comments.push(Comment {
            comment_id,
            username,
            userurl,
            useravatar,
            create_date: create_date
                .assume_offset(utc_offset)
                .format(&output_format)?,
            edit_date: edit_date.assume_offset(utc_offset).format(&output_format)?,
            comment,
        });
    }

    Ok(comments)
}

pub fn get_blog_id_by_comment_id(sql_ctx: SQLCtx, cid: &str) -> Result<String, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(cid)?;

    let rows = conn.query_with_params_rows(
        "SELECT blog_post_id FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NULL",
        &params,
    )?;

    if let Some(rows) = rows
        && rows.len() == 1
        && rows[0].len() == 1
    {
        match &rows[0][0] {
            msql_ffi::MSQLValueEnum::Error => {
                Err("Internal Error blog id not valid in query".into())
            }
            msql_ffi::MSQLValueEnum::Null => {
                Err("Internal Error blog id not valid in query".into())
            }
            msql_ffi::MSQLValueEnum::Int64(_) => {
                Err("Internal Error blog id not valid in query".into())
            }
            msql_ffi::MSQLValueEnum::UInt64(_) => {
                Err("Internal Error blog id not valid in query".into())
            }
            msql_ffi::MSQLValueEnum::String(s) => Ok(s.to_owned()),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => {
                Err("Internal Error blog id not valid in query".into())
            }
        }
    } else {
        Err("Internal Error failed to query blog id by comment id".into())
    }
}

pub fn cleanup_logins(sql_ctx: SQLCtx, minutes_timeout: u64) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_uint64(minutes_timeout);

    conn.query_with_params_drop(
        "DELETE FROM LOGIN WHERE TIMESTAMPDIFF(MINUTE, login_date, CURRENT_TIMESTAMP) > ?",
        &params,
    )?;

    Ok(())
}

pub fn add_login(
    sql_ctx: SQLCtx,
    ip: Option<&str>,
    user_id: u64,
    username: &str,
    userlogin: &str,
    userurl: &str,
    useravatar: &str,
) -> Result<String, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut id: String = uuid::Uuid::new_v4().to_string();

    loop {
        let mut params = MSQLParamsWrapper::new();
        params.append_str(&id)?;
        let ret = conn.query_with_params_rows("SELECT id FROM LOGIN WHERE id = ?", &params)?;
        if ret.is_none() {
            break;
        } else {
            id = uuid::Uuid::new_v4().to_string();
        }
    }

    let mut params = MSQLParamsWrapper::new();
    params.append_str(&id)?;
    if let Some(ip) = ip {
        params.append_str(ip)?;
    } else {
        params.append_null();
    }
    params.append_uint64(user_id);
    params.append_str(username)?;
    params.append_str(userlogin)?;
    params.append_str(userurl)?;
    params.append_str(useravatar)?;

    conn.query_with_params_drop("INSERT INTO LOGIN (id, ip, user_id, username, userlogin, userurl, useravatar) VALUES (?, ?, ?, ?, ? ,? ,?)", &params)?;

    Ok(id)
}

pub fn check_logged_in(
    sql_ctx: SQLCtx,
    id: &str,
    ip: Option<&str>,
) -> Result<Option<LoginInfo>, Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(id)?;
    if let Some(ip) = ip {
        params.append_str(ip)?;
    }
    let ret = if ip.is_some() {
        conn.query_with_params_rows("SELECT id, ip, user_id, username, userlogin, userurl, useravatar FROM LOGIN WHERE id = ? AND ip = ?", &params)?
    } else {
        conn.query_with_params_rows(
            "SELECT id, ip, user_id, username, userurl, useravatar FROM LOGIN WHERE id = ?",
            &params,
        )?
    };

    if let Some(rows) = ret {
        if rows[0].len() != 7 {
            return Err(
                "check_logged_in: Failed due to invalid number of cols returned by query!".into(),
            );
        }
        let user_id = match &rows[0][0] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid user_id from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid user_id from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid user_id from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid user_id from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => return Err("Invalid user_id from db!".into()),
        };
        let user_ip = match &rows[0][1] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid user_ip from db!".into()),
            msql_ffi::MSQLValueEnum::Null => None,
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid user_ip from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid user_ip from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => Some(s.to_owned()),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => return Err("Invalid user_ip from db!".into()),
        };
        let user_github_id = match &rows[0][2] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid user_github_id from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid user_github_id from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(i) => *i as u64,
            msql_ffi::MSQLValueEnum::UInt64(u) => *u,
            msql_ffi::MSQLValueEnum::String(_) => {
                return Err("Invalid user_github_id from db!".into());
            }
            msql_ffi::MSQLValueEnum::DoubleF64(_) => {
                return Err("Invalid user_github_id from db!".into());
            }
        };
        let username = match &rows[0][3] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => return Err("Invalid username from db!".into()),
        };
        let userlogin = match &rows[0][4] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid username from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => return Err("Invalid username from db!".into()),
        };
        let userurl = match &rows[0][5] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid userurl from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid userurl from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid userurl from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid userurl from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => return Err("Invalid userurl from db!".into()),
        };
        let useravatar = match &rows[0][6] {
            msql_ffi::MSQLValueEnum::Error => return Err("Invalid useravatar from db!".into()),
            msql_ffi::MSQLValueEnum::Null => return Err("Invalid useravatar from db!".into()),
            msql_ffi::MSQLValueEnum::Int64(_) => return Err("Invalid useravatar from db!".into()),
            msql_ffi::MSQLValueEnum::UInt64(_) => return Err("Invalid useravatar from db!".into()),
            msql_ffi::MSQLValueEnum::String(s) => s.to_owned(),
            msql_ffi::MSQLValueEnum::DoubleF64(_) => {
                return Err("Invalid useravatar from db!".into());
            }
        };
        Ok(Some(LoginInfo {
            user_id,
            user_ip,
            user_github_id,
            username,
            userlogin,
            userurl,
            useravatar,
        }))
    } else {
        Ok(None)
    }
}

pub fn logout(sql_ctx: SQLCtx, id: &str) -> Result<(), Error> {
    let conn: Arc<Mutex<MSQLWrapper>> = sql_ctx.try_into()?;
    let mut conn = conn
        .try_lock()
        .map_err(|_| -> Error { "Failed to get unique connection".into() })?;

    let mut params = MSQLParamsWrapper::new();
    params.append_str(id)?;

    conn.query_with_params_drop("DELETE FROM LOGIN WHERE id = ?", &params)?;

    Ok(())
}
