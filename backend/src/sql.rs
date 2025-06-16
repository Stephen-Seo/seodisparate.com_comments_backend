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
            creation_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            INDEX creation_date_index USING BTREE (creation_date),
            edit_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            comment TEXT NOT NULL
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
    Err(Error::from("Unimplmented"))
}
