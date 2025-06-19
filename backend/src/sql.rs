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

use std::thread::sleep;
use std::time::Duration;

use crate::error::Error;
use mysql::prelude::*;
use mysql::*;
use serde::Serialize;
use time::{PrimitiveDateTime, UtcDateTime, UtcOffset, format_description};

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

#[derive(Debug)]
struct PreProcessedComment {
    comment_id: String,
    username: String,
    userurl: String,
    useravatar: String,
    create_date: Result<String, time::error::Format>,
    edit_date: Result<String, time::error::Format>,
    comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PseudoComment {
    user_id: u64,
    username: String,
    userurl: String,
    useravatar: String,
    blog_post_id: String,
    comment_id: String,
}

pub fn set_up_sql_db(conn_str: &str) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS COMMENT (
            uuid CHAR(36) PRIMARY KEY,
            blog_post_id TINYTEXT NOT NULL,
            INDEX blog_post_id_index USING HASH (blog_post_id),
            user_id BIGINT NOT NULL,
            INDEX user_id_index USING HASH (user_id),
            username TINYTEXT NOT NULL,
            userurl TINYTEXT NOT NULL,
            useravatar TINYTEXT NOT NULL,
            creation_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            INDEX creation_date_index USING BTREE (creation_date),
            edit_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            comment TEXT NOT NULL
        )",
    )?;

    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS PSEUDO_COMMENT (
            uuid CHAR(36) PRIMARY KEY,
            state CHAR(36) NOT NULL UNIQUE,
            user_id BIGINT NOT NULL,
            username TINYTEXT NOT NULL,
            userurl TINYTEXT NOT NULL,
            useravatar TINYTEXT NOT NULL,
            blog_post_id TINYTEXT,
            comment_id TINYTEXT,
            date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            date2 DATETIME NOT NULL DEFAULT ADDTIME(CURRENT_TIMESTAMP, '00:00:01'),
            PERIOD FOR date_period(date, date2)
        )",
    )?;

    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS GITHUB_RNG (
            uuid CHAR(36) PRIMARY KEY,
            date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            date2 DATETIME NOT NULL DEFAULT ADDTIME(CURRENT_TIMESTAMP, '00:00:01'),
            PERIOD FOR date_period(date, date2)
        )",
    )?;

    Ok(())
}

pub fn has_psuedo_commment_with_state(conn: &mut PooledConn, state: &str) -> Result<bool, Error> {
    Ok(conn
        .exec_first::<String, &'static str, (&str,)>(
            "SELECT state FROM PSEUDO_COMMENT WHERE state = ?",
            (state,),
        )?
        .is_some())
}

pub fn create_rng_uuid(conn_str: &str) -> Result<String, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM GITHUB_RNG
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 60 MINUTE)",
    )?;

    let mut rng_uuid = uuid::Uuid::new_v4();

    while has_psuedo_commment_with_state(&mut conn, &rng_uuid.to_string())? {
        rng_uuid = uuid::Uuid::new_v4();
    }

    let rng_uuid_string = rng_uuid.to_string();

    conn.exec_drop(
        r"INSERT INTO GITHUB_RNG (uuid) VALUES (?)",
        (&rng_uuid_string,),
    )?;

    Ok(rng_uuid_string)
}

pub fn check_rng_uuid(conn_str: &str, uuid: &str) -> Result<bool, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM GITHUB_RNG
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 60 MINUTE)",
    )?;

    let ret: Option<String> =
        conn.exec_first(r"SELECT uuid FROM GITHUB_RNG WHERE uuid = ?", (uuid,))?;

    Ok(ret.is_some())
}

pub fn check_remove_rng_uuid(conn_str: &str, uuid: &str) -> Result<bool, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM GITHUB_RNG
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 60 MINUTE)",
    )?;

    let ret: Option<String> =
        conn.exec_first(r"SELECT uuid FROM GITHUB_RNG WHERE uuid = ?", (uuid,))?;

    if let Some(ret_uuid) = &ret {
        conn.exec_drop(r"DELETE FROM GITHUB_RNG WHERE uuid = ?", (ret_uuid,))?;
    }

    Ok(ret.is_some())
}

pub fn add_pseudo_comment_data(
    conn_str: &str,
    state: &str,
    user_id: u64,
    user_name: &str,
    user_url: &str,
    user_avatar_url: &str,
    blog_post_id: Option<&str>,
    comment_id: Option<&str>,
) -> Result<String, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM PSEUDO_COMMENT
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 60 MINUTE)",
    )?;

    let uuid_string: String;

    loop {
        let uuid = uuid::Uuid::new_v4();
        let row_opt: Option<Row> = conn.exec_first(
            "SELECT uuid FROM PSEUDO_COMMENT WHERE uuid = ?",
            (uuid.to_string(),),
        )?;
        if row_opt.is_none() {
            uuid_string = uuid.to_string();
            break;
        }
    }

    conn.exec_drop(r"INSERT INTO PSEUDO_COMMENT (uuid, state, user_id, username, userurl, useravatar, blog_post_id, comment_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", (&uuid_string, state, user_id, user_name, user_url, user_avatar_url, blog_post_id, comment_id))?;

    Ok(uuid_string)
}

pub fn add_comment(conn_str: &str, state: &str, comment: &str) -> Result<String, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let pseudo_comment = conn.exec_map(
        "SELECT user_id, username, userurl, useravatar, blog_post_id FROM PSEUDO_COMMENT WHERE state = ?",
        (state,),
        |(user_id, username, userurl, useravatar, blog_post_id)| PseudoComment {
            user_id,
            username,
            userurl,
            useravatar,
            blog_post_id,
            comment_id: String::new(),
        },
    )?;

    if pseudo_comment.is_empty() {
        return Err(Error::from(
            "PsuedoComment not found, Commentor not authenticated or timed out!",
        ));
    }

    let mut combined: String = pseudo_comment[0].blog_post_id.clone();
    combined.push_str(&pseudo_comment[0].user_id.to_string());
    let utc_time: UtcDateTime = UtcDateTime::now();
    combined.push_str(&utc_time.to_string());

    let namespace = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, "seodisparate.com".as_bytes());
    let mut uuid = uuid::Uuid::new_v5(&namespace, combined.as_bytes());
    let mut uuid_str = uuid.to_string();

    loop {
        let row_opt: Option<Row> =
            conn.exec_first("SELECT uuid FROM COMMENT WHERE uuid = ?", (&uuid_str,))?;
        if row_opt.is_some() {
            sleep(Duration::from_secs(1));
            let utc_time = UtcDateTime::now();
            combined = pseudo_comment[0].blog_post_id.clone();
            combined.push_str(&pseudo_comment[0].user_id.to_string());
            combined.push_str(&utc_time.to_string());
            uuid = uuid::Uuid::new_v5(&namespace, combined.as_bytes());
            uuid_str = uuid.to_string();
        } else {
            break;
        }
    }

    conn.exec_drop("INSERT INTO COMMENT (uuid, blog_post_id, user_id, username, userurl, useravatar, comment) VALUES (?, ?, ?, ?, ?, ?, ?)", (uuid_str, &pseudo_comment[0].blog_post_id, pseudo_comment[0].user_id, &pseudo_comment[0].username, &pseudo_comment[0].userurl, &pseudo_comment[0].useravatar, comment))?;

    conn.exec_drop("DELETE FROM PSEUDO_COMMENT WHERE state = ?", (state,))?;

    Ok(pseudo_comment[0].blog_post_id.to_owned())
}

pub fn check_edit_comment_auth(conn_str: &str, cid: &str, uid: &str) -> Result<bool, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let row_opt: Option<Row> = conn.exec_first(
        "SELECT uuid FROM COMMENT WHERE uuid = ? AND user_id = ?",
        (cid, uid),
    )?;

    Ok(row_opt.is_some())
}

pub fn get_comment_text(conn_str: &str, cid: &str) -> Result<String, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let mut row: Row = conn
        .exec_first("SELECT comment from COMMENT WHERE uuid = ?", (cid,))?
        .ok_or(Error::from("Editing comment: Comment not found!"))?;

    row.take::<String, usize>(0).ok_or(Error::from(
        "Editing comment: Comment failed to convert to String!",
    ))
}

pub fn edit_comment(conn_str: &str, state: &str, comment: &str) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let pseudo_comment = conn.exec_map(
        "SELECT user_id, username, userurl, useravatar, comment_id FROM PSEUDO_COMMENT WHERE state = ?",
        (state,),
        |(user_id, username, userurl, useravatar, comment_id)| PseudoComment {
            user_id,
            username,
            userurl,
            useravatar,
            blog_post_id: String::new(),
            comment_id,
        },
    )?;

    if pseudo_comment.is_empty() {
        return Err(Error::from(
            "PsuedoComment not found, Commentor not authenticated or timed out!",
        ));
    }

    conn.exec_drop("UPDATE COMMENT SET username = ?, userurl = ?, useravatar = ?, edit_date = CURRENT_TIMESTAMP, comment = ? WHERE uuid = ?", (&pseudo_comment[0].username, &pseudo_comment[0].userurl, &pseudo_comment[0].useravatar, comment, &pseudo_comment[0].comment_id))?;

    conn.exec_drop("DELETE FROM PSEUDO_COMMENT WHERE state = ?", (state,))?;

    Ok(())
}

pub fn try_delete_comment(conn_str: &str, cid: &str, uid: u64) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.exec_drop(
        "DELETE FROM COMMENT WHERE uuid = ? AND user_id = ?",
        (cid, uid),
    )?;

    Ok(())
}

pub fn get_comments_per_blog_id(conn_str: &str, blog_id: &str) -> Result<Vec<Comment>, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let utc_offset = UtcOffset::current_local_offset()?;

    let format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
    )?;

    let pre_proc_comments = conn.exec_map(
        "SELECT uuid, username, userurl, useravatar, creation_date, edit_date, comment FROM COMMENT WHERE blog_post_id = ? ORDER BY creation_date",
        (blog_id,), |(uuid, username, userurl, useravatar, creation_date, edit_date, comment)| {
            let create_time: PrimitiveDateTime = creation_date;
            let edit_time: PrimitiveDateTime = edit_date;
            PreProcessedComment {
                comment_id: uuid,
                username,
                userurl,
                useravatar,
                create_date: create_time.assume_offset(utc_offset).format(&format),
                edit_date: edit_time.assume_offset(utc_offset).format(&format),
                comment,
            }
        }
    )?;

    let mut comments = Vec::new();

    for pre in pre_proc_comments {
        comments.push(Comment {
            comment_id: pre.comment_id,
            username: pre.username,
            userurl: pre.userurl,
            useravatar: pre.useravatar,
            create_date: pre.create_date?,
            edit_date: pre.edit_date?,
            comment: pre.comment,
        });
    }

    Ok(comments)
}
