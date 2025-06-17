use crate::error::Error;
use mysql::prelude::*;
use mysql::*;
use time::UtcDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PseudoComment {
    user_id: u64,
    username: String,
    userurl: String,
    useravatar: String,
    blog_post_id: String,
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
            user_id BIGINT NOT NULL,
            username TINYTEXT NOT NULL,
            userurl TINYTEXT NOT NULL,
            useravatar TINYTEXT NOT NULL,
            blog_post_id TINYTEXT NOT NULL,
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

pub fn create_rng_uuid(conn_str: &str) -> Result<String, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM GITHUB_RNG
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 30 MINUTE))",
    )?;

    let rng_uuid = uuid::Uuid::new_v4();
    let rng_uuid_string = rng_uuid.to_string();

    conn.exec_drop(
        r"INSERT INTO GITHUB_RNG (uuid) VALUES (?)",
        (&rng_uuid_string,),
    )?;

    Ok(rng_uuid_string)
}

pub fn check_remove_rng_uuid(conn_str: &str, uuid: &str) -> Result<bool, Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let ret: Option<String> =
        conn.exec_first(r"SELECT uuid FROM GITHUB_RNG WHERE uuid = ?", (uuid,))?;

    if let Some(ret_uuid) = &ret {
        conn.exec_drop(r"DELETE FROM GITHUB_RNG WHERE uuid = ?", (ret_uuid,))?;
    }

    Ok(ret.is_some())
}

pub fn add_pseudo_comment_data(
    conn_str: &str,
    uuid: &str,
    user_id: u64,
    user_name: &str,
    user_url: &str,
    user_avatar_url: &str,
    blog_post_id: &str,
) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM PSEUDO_COMMENT
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 60 MINUTE))",
    )?;

    conn.exec_drop(r"INSERT INTO PSEUDO_COMMENT (uuid, user_id, username, userurl, useravatar, blog_post_id) VALUES (?, ?, ?, ?, ?)", (uuid, user_id, user_name, user_url, user_avatar_url, blog_post_id))?;

    Ok(())
}

pub fn add_comment(conn_str: &str, state: &str, comment: &str) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    let pseudo_comment = conn.exec_map(
        "SELECT user_id, username, userurl, useravatar, blog_post_id FROM PSEUDO_COMMENT WHERE uuid = ?",
        (state,),
        |(user_id, username, userurl, useravatar, blog_post_id)| PseudoComment {
            user_id,
            username,
            userurl,
            useravatar,
            blog_post_id,
        },
    )?;

    if pseudo_comment.is_empty() {
        return Err(Error::from("Commentor not authenticated or timed out!"));
    }

    let mut combined: String = pseudo_comment[0].blog_post_id.clone();
    combined.push_str(&pseudo_comment[0].user_id.to_string());
    let utc_time: UtcDateTime = UtcDateTime::now();
    combined.push_str(&utc_time.to_string());

    let namespace = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, "seodisparate.com".as_bytes());
    let uuid = uuid::Uuid::new_v5(&namespace, combined.as_bytes());
    let uuid_str = uuid.to_string();

    conn.exec_drop("INSERT INTO COMMENT (uuid, blog_post_id, user_id, username, userurl, useravatar, comment) VALUES (?, ?, ?, ?, ?, ?)", (uuid_str, &pseudo_comment[0].blog_post_id, pseudo_comment[0].user_id, &pseudo_comment[0].username, &pseudo_comment[0].userurl, &pseudo_comment[0].useravatar, comment))?;

    conn.exec_drop("DELETE FROM PSEUDO_COMMENT WHERE uuid = ?", (state,))?;

    Ok(())
}
