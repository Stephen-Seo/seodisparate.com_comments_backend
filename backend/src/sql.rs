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

use crate::error::Error;
use mysql::prelude::*;
use mysql::*;
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
pub struct PseudoComment {
    pub user_id: u64,
    pub username: String,
    pub userurl: String,
    pub useravatar: String,
    pub blog_post_id: String,
    pub comment_id: String,
}

pub fn set_up_sql_db(pool: &Pool, db_name: &str) -> Result<(), Error> {
    let mut conn = pool.get_conn()?;

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

    {
        // Migrate from COMMENT to COMMENT2 if exists.
        let row: Option<Row> = conn.exec_first(
            "SELECT * FROM information_schema.tables WHERE table_schema = ? AND table_name = 'COMMENT'",
            (db_name,),
        )?;

        if row.is_some() {
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

pub fn has_psuedo_commment_with_state(conn: &mut PooledConn, state: &str) -> Result<bool, Error> {
    Ok(conn
        .exec_first::<String, &'static str, (&str,)>(
            "SELECT uuid FROM COMMENT2 WHERE uuid = ?",
            (state,),
        )?
        .is_some())
}

pub fn create_rng_uuid(pool: &Pool, uuid: Option<&str>) -> Result<String, Error> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    let mut rng_uuid = uuid::Uuid::new_v4();

    while has_psuedo_commment_with_state(&mut conn, &rng_uuid.to_string())? {
        rng_uuid = uuid::Uuid::new_v4();
    }

    let rng_uuid_string = rng_uuid.to_string();

    if let Some(uuid_str) = uuid {
        conn.exec_drop(
            r"UPDATE COMMENT2 SET state = ? WHERE uuid = ? AND timeout_date IS NULL",
            (&rng_uuid_string, uuid_str),
        )?;
        conn.exec_first::<String, &'static str, (&str,)>(
            r"SELECT state FROM COMMENT2 WHERE state IS NOT NULL AND uuid = ?",
            (uuid_str,),
        )?
        .ok_or(Error::Generic(
            "Failed to add state to existing comment!".into(),
        ))?;
    } else {
        conn.exec_drop(
            r"INSERT INTO COMMENT2 (uuid) VALUES (?)",
            (&rng_uuid_string,),
        )?;
    }

    Ok(rng_uuid_string)
}

pub fn check_rng_uuid(pool: &Pool, uuid: &str, state: Option<&str>) -> Result<bool, Error> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    if let Some(state) = state {
        let ret: Option<String> = conn.exec_first(
            r"SELECT uuid FROM COMMENT2 WHERE uuid = ? AND state = ? AND timeout_date IS NULL",
            (uuid, state),
        )?;

        Ok(ret.is_some())
    } else {
        let ret: Option<String> = conn.exec_first(
            r"SELECT uuid FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NOT NULL",
            (uuid,),
        )?;

        Ok(ret.is_some())
    }
}

pub fn add_pseudo_comment_data(
    pool: &Pool,
    state: &str,
    user_id: u64,
    user_name: &str,
    user_url: &str,
    user_avatar_url: &str,
    blog_post_id: Option<&str>,
    comment_id: Option<&str>,
) -> Result<String, Error> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    if let Some(blog_id) = blog_post_id {
        conn.exec_first::<String, &'static str, (&str,)>(
            r"SELECT uuid FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NOT NULL",
            (state,),
        )?
        .ok_or(Error::Generic("Timed out creating comment!".into()))?;
        conn.exec_drop(r"UPDATE COMMENT2 SET user_id=?, username=?, userurl=?, useravatar=?, blog_post_id=? WHERE uuid = ?", (user_id, user_name, user_url, user_avatar_url, blog_id, state))?;
    } else if let Some(comment_id) = comment_id {
        conn.exec_first::<String, &'static str, (&str, &str)>(
            r"SELECT uuid FROM COMMENT2 WHERE uuid = ? AND state = ? AND timeout_date IS NULL",
            (comment_id, state),
        )?
        .ok_or(Error::Generic("Timed out creating comment!".into()))?;
        conn.exec_drop(
            r"UPDATE COMMENT2 SET user_id=?, username=?, userurl=?, useravatar=?, state=NULL WHERE uuid = ?",
            (user_id, user_name, user_url, user_avatar_url, comment_id),
        )?;
    }

    Ok(state.to_string())
}

pub fn add_comment(pool: &Pool, state: &str, comment: &str) -> Result<PseudoComment, Error> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM COMMENT2 WHERE timeout_date IS NOT NULL AND TIMESTAMPDIFF(MINUTE, timeout_date, CURRENT_TIMESTAMP) > 60"
    )?;

    conn.exec_first::<String, &'static str, (&str,)>(
        r"SELECT uuid FROM COMMENT2 WHERE uuid = ?",
        (state,),
    )?
    .ok_or(Error::Generic("Timed out creating comment!".into()))?;

    conn.exec_drop(
        "UPDATE COMMENT2 SET timeout_date=NULL, comment=? WHERE uuid = ?",
        (comment, state),
    )?;

    let pseudo_comment = conn.exec_map(
        "SELECT user_id, username, userurl, useravatar, blog_post_id FROM COMMENT2 WHERE uuid = ?",
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

    Ok(pseudo_comment[0].clone())
}

pub fn check_edit_comment_auth(pool: &Pool, cid: &str, uid: &str) -> Result<bool, Error> {
    let mut conn = pool.get_conn()?;

    let row_opt: Option<Row> = conn.exec_first(
        "SELECT uuid FROM COMMENT2 WHERE uuid = ? AND user_id = ?",
        (cid, uid),
    )?;

    Ok(row_opt.is_some())
}

pub fn get_comment_text(pool: &Pool, cid: &str) -> Result<String, Error> {
    let mut conn = pool.get_conn()?;

    let mut row: Row = conn
        .exec_first("SELECT comment from COMMENT2 WHERE uuid = ?", (cid,))?
        .ok_or(Error::from("Editing comment: Comment not found!"))?;

    row.take::<String, usize>(0).ok_or(Error::from(
        "Editing comment: Comment failed to convert to String!",
    ))
}

pub fn edit_comment(pool: &Pool, uuid: &str, comment: &str) -> Result<(), Error> {
    let mut conn = pool.get_conn()?;

    conn.exec_drop(
        "UPDATE COMMENT2 SET edit_date = CURRENT_TIMESTAMP, comment = ? WHERE uuid = ?",
        (comment, uuid),
    )?;

    Ok(())
}

pub fn try_delete_comment(pool: &Pool, cid: &str, uid: u64) -> Result<(), Error> {
    let mut conn = pool.get_conn()?;

    conn.exec_drop(
        "DELETE FROM COMMENT2 WHERE uuid = ? AND user_id = ?",
        (cid, uid),
    )?;

    Ok(())
}

pub fn try_delete_comment_id_only(pool: &Pool, cid: &str) -> Result<(), Error> {
    let mut conn = pool.get_conn()?;

    conn.exec_drop("DELETE FROM COMMENT2 WHERE uuid = ?", (cid,))?;

    Ok(())
}

pub fn get_comments_per_blog_id(pool: &Pool, blog_id: &str) -> Result<Vec<Comment>, Error> {
    let mut conn = pool.get_conn()?;

    let utc_offset = UtcOffset::current_local_offset()?;

    let format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
    )?;

    let pre_proc_comments = conn.exec_map(
        "SELECT uuid, username, userurl, useravatar, creation_date, edit_date, comment FROM COMMENT2 WHERE blog_post_id = ? ORDER BY creation_date",
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

pub fn get_blog_id_by_comment_id(pool: &Pool, cid: &str) -> Result<String, Error> {
    let mut conn = pool.get_conn()?;

    let mut row: Row = conn
        .exec_first(
            r"SELECT blog_post_id FROM COMMENT2 WHERE uuid = ? AND timeout_date IS NULL",
            (cid,),
        )?
        .ok_or(Error::Generic("Comment not found!".into()))?;

    Ok(row
        .take(0)
        .ok_or(Error::Generic("Internal Error, Comment not found!".into()))?)
}
