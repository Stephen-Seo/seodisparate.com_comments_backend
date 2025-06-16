use crate::error::Error;
use mysql::prelude::*;
use mysql::*;

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
) -> Result<(), Error> {
    let pool = Pool::new(conn_str)?;

    let mut conn = pool.get_conn()?;

    conn.query_drop(
        r"DELETE FROM PSEUDO_COMMENT
        FOR PORTION OF date_period
        FROM '0-0-0' TO SUBDATE(CURRENT_TIMESTAMP, INTERVAL 30 MINUTE))",
    )?;

    conn.exec_drop(r"INSERT INTO PSEUDO_COMMENT (uuid, user_id, username, userurl, useravatar) VALUES (?, ?, ?, ?, ?)", (uuid, user_id, user_name, user_url, user_avatar_url))?;

    Ok(())
}
