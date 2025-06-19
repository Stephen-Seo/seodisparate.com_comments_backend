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

mod arg_parse;
mod config;
mod error;
mod sql;

use std::time::Duration;

use error::Error;
use reqwest::Url;
use salvo::prelude::*;
use tokio::time::sleep;

pub const COMMON_CSS: &str = r#"
    body {
        color: #FFF;
        background-color: #444;
    }
    a {
        color: #8F8;
    }
    textarea {
        color: #FFF;
        background-color: #222;
    }
    button {
        color: #FFF;
        background-color: #333;
    }
"#;

pub const WRITE_COMMENT_PAGE: &str = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="utf-8">
        <title>Write a Comment - {BLOG_ID}</title>
        <style>{COMMON_CSS}</style>
    </head>
    <body>
        <h1>Write a Comment</h1>
        <div>
            <p>Note that this site reserves the right to delete any comment on
            the grounds that it is spam/hateful/etc. Please use common sense,
            and please be courteous to others, even when contrary.</p>
            <p>You can edit/delete your comment after posting it.</p>
        </div><br>
        <img width="64" height="64" src="{USER_AVATAR_URL}" /> <b>{USER_NAME}</b> <a href="{USER_PROFILE}">(User Profile)</a><br>
        <textarea id="comment_text" name="comment_text" rows="10" cols="50" autofocus=true maxlength="65000"></textarea><br>
        <button id="comment_submit_button">Submit</button><br>
        <p id="status_paragraph"></p>
        <script>
            "use strict;"

            async function submit_comment(json) {
                const response = await fetch("{BASE_URL}/submit_comment",
                    {
                        method: "POST",
                        body: json,
                        headers: {
                            "Content-Type": "application/json",
                        },
                    }
                );
                if (!response.ok) {
                    let status_p = document.getElementById("status_paragraph");
                    status_p.innerText = "ERROR: Failed to submit comment!";
                    throw new Error(`Response status: ${response.status}`);
                } else {
                    window.location = "{BLOG_URL}";
                }
            }

            window.addEventListener("load", (event) => {
                let button = document.getElementById("comment_submit_button");
                let textarea = document.getElementById("comment_text");

                button.addEventListener("click", (e) => {
                    let submit_obj = {};
                    submit_obj.comment_text = textarea.value;
                    submit_obj.state = "{STATE_STRING}";
                    let submit_json = JSON.stringify(submit_obj);
                    submit_comment(submit_json);
                });
            });
        </script>
    </body>
    </html>
"#;

pub const EDIT_COMMENT_PAGE: &str = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="utf-8">
        <title>Edit a Comment</title>
        <style>{COMMON_CSS}</style>
    </head>
    <body>
        <h1>Edit a Comment</h1>
        <div>
            <p>Note that this site reserves the right to delete any comment on
            the grounds that it is spam/hateful/etc. Please use common sense,
            and please be courteous to others, even when contrary.</p>
            <p>You can edit/delete your comment after editing it.</p>
        </div><br>
        <img width="64" height="64" src="{USER_AVATAR_URL}" /> <b>{USER_NAME}</b> <a href="{USER_PROFILE}">(User Profile)</a><br>
        <textarea id="comment_text" name="comment_text" rows="10" cols="50" autofocus=true maxlength="65000">Loading...</textarea><br>
        <button id="comment_submit_button">Submit</button><br>
        <p id="status_paragraph"></p>
        <script>
            "use strict;"

            async function populate_textarea(ta, cid) {
                const response = await fetch("{BASE_URL}/get_comment?comment_id=" + cid);
                if (response.ok) {
                    ta.value = await response.text();
                } else {
                    ta.value = "Error: Failed to load comment!";
                }
            }

            async function submit_comment(json) {
                const response = await fetch("{BASE_URL}/submit_edit_comment",
                    {
                        method: "POST",
                        body: json,
                        headers: {
                            "Content-Type": "application/json",
                        },
                    }
                );
                if (!response.ok) {
                    let status_p = document.getElementById("status_paragraph");
                    status_p.innerText = "Error: Failed to edit comment!";
                    throw new Error(`Response status: ${response.status}`);
                } else {
                    window.location = "{BLOG_URL}";
                }
            }

            window.addEventListener("load", (event) => {
                let button = document.getElementById("comment_submit_button");
                let textarea = document.getElementById("comment_text");

                populate_textarea(textarea, "{COMMENT_ID}");

                button.addEventListener("click", (e) => {
                    let submit_obj = {};
                    submit_obj.comment_text = textarea.value;
                    submit_obj.state = "{STATE_STRING}";
                    let submit_json = JSON.stringify(submit_obj);
                    submit_comment(submit_json);
                });
            });
        </script>
    </body>
    </html>
"#;

#[derive(Default, Clone, Debug)]
struct Config {
    db_conn_string: String,
    oauth_user: String,
    oauth_token: String,
    base_url: String,
    allowed_urls: Vec<String>,
    allowed_bids: Vec<String>,
    user_agent: String,
}

#[handler]
async fn root_handler(res: &mut Response) {
    res.body(format!(
        "<html><head><style>{}</style></head><body><h1>Welcome</h1></body></html>",
        COMMON_CSS
    ));
}

#[handler]
async fn comment_text_get(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), Error> {
    let salvo_conf = depot.obtain::<Config>().unwrap();

    let comment_id: String = req
        .try_query("comment_id")
        .map_err(Error::err_to_client_err)?;

    let comment_text: String = sql::get_comment_text(&salvo_conf.db_conn_string, &comment_id)?;

    res.body(comment_text);

    Ok(())
}

#[handler]
async fn login_to_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), error::Error> {
    let blog_id: String = req.try_query("blog_id").map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(Error::err_to_client_err)?;
    let salvo_conf = depot.obtain::<Config>().unwrap();
    let is_allowed_url: bool = salvo_conf.allowed_urls.iter().fold(false, |acc, val| {
        if acc { acc } else { blog_url.starts_with(val) }
    });
    if !is_allowed_url {
        eprintln!("Client blog_url is invalid! {}", blog_url);
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }
    let is_allowed_bid: bool = salvo_conf
        .allowed_bids
        .iter()
        .fold(false, |acc, val| if acc { acc } else { &blog_id == val });
    if !is_allowed_bid {
        eprintln!("Client blog id is invalid! {}", blog_id);
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }
    let uuid = sql::create_rng_uuid(&salvo_conf.db_conn_string)?;
    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_make_comment", salvo_conf.base_url),
        &[("blog_id", blog_id), ("blog_url", blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;
    let github_api_url = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("state", uuid.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ],
    )
    .map_err(|_| error::Error::from("Failed to parse github api url!"))?;
    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "{}";
            }}, 3000);
        "#,
        github_api_url.as_str()
    );

    res.body(format!(
        r#"<html><head><style>{}</style></head><body>
        <b>Redirecting to Github for Authentication...</b>
        <script>
        {}
        </script>
        </body></html>"#,
        COMMON_CSS, script
    ));

    Ok(())
}

#[handler]
async fn github_auth_make_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), error::Error> {
    let blog_id: String = req
        .try_query("blog_id")
        .map_err(error::Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(error::Error::err_to_client_err)?;
    let state: String = req
        .try_query("state")
        .map_err(error::Error::err_to_client_err)?;
    let code: String = req
        .try_query("code")
        .map_err(error::Error::err_to_client_err)?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    let is_state_valid = sql::check_rng_uuid(&salvo_conf.db_conn_string, &state)?;
    if !is_state_valid {
        eprintln!("State is invalid (timed out?)!\n");
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request (took too long to verify)</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }

    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_make_comment", salvo_conf.base_url),
        &[("blog_id", &blog_id), ("blog_url", &blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;

    let client = reqwest::Client::builder();
    let client = client.user_agent(&salvo_conf.user_agent).build()?;
    let g_res = client
        .post("https://github.com/login/oauth/access_token")
        .query(&[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("client_secret", salvo_conf.oauth_token.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;

    let json: serde_json::Value = g_res.json().await?;
    let access_token = json.get("access_token").ok_or(error::Error::from(
        "Failed to parse access_token from response from Github!",
    ))?;
    let access_token_str: &str = access_token
        .as_str()
        .ok_or(Error::from("Github access_token was not a string!"))?;

    let mut reqw_resp: Option<reqwest::Response> = None;
    for _idx in 0..3 {
        let ret = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", &format!("Bearer {}", access_token_str))
            .header("X-Github-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(Error::from);
        if ret.is_ok() {
            let ret = ret?.error_for_status();
            if ret.is_ok() {
                reqw_resp = Some(ret?);
                break;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        } else {
            sleep(Duration::from_secs(3)).await;
        }
    }
    let user_info: serde_json::Value = reqw_resp
        .ok_or(Error::from("Failed to get user info via oauth token!"))?
        .json()
        .await?;

    let user_id: u64 = user_info
        .get("id")
        .ok_or(error::Error::from("Failed to parse user info id!"))?
        .to_string()
        .parse()?;

    let mut user_name: Option<&serde_json::Value> = user_info.get("name");
    let user_name_str: String;

    if let Some(user_name_inner) = user_name {
        if user_name_inner.is_string() {
            user_name_str = user_name_inner
                .as_str()
                .ok_or(error::Error::from("Failed to parse user info name!"))?
                .to_owned();
        } else {
            user_name = user_info.get("login");
            user_name_str = user_name
                .ok_or(error::Error::from("User has no name or login!"))?
                .as_str()
                .ok_or(error::Error::from("Failed to parse user info login!"))?
                .to_owned();
        }
    } else {
        user_name = user_info.get("login");
        user_name_str = user_name
            .ok_or(error::Error::from("User has no name or login!"))?
            .as_str()
            .ok_or(error::Error::from("Failed to parse user info login!"))?
            .to_owned();
    }

    let user_url = user_info
        .get("html_url")
        .ok_or(error::Error::from("Failed to parse user info profile url!"))?
        .as_str()
        .ok_or(error::Error::from("Failed to parse user info profile url!"))?;

    let user_avatar_url = user_info
        .get("avatar_url")
        .ok_or(error::Error::from(
            "Failed to parse user info profile avatar url!",
        ))?
        .as_str()
        .ok_or(error::Error::from(
            "Failed to parse user info profile avatar url!",
        ))?;

    sql::add_pseudo_comment_data(
        &salvo_conf.db_conn_string,
        &state,
        user_id,
        &user_name_str,
        user_url,
        user_avatar_url,
        Some(&blog_id),
        None,
    )?;

    res.body(
        WRITE_COMMENT_PAGE
            .replace("{BLOG_ID}", &blog_id)
            .replace("{COMMON_CSS}", COMMON_CSS)
            .replace("{USER_AVATAR_URL}", user_avatar_url)
            .replace("{USER_NAME}", &user_name_str)
            .replace("{USER_PROFILE}", user_url)
            .replace("{BASE_URL}", &salvo_conf.base_url)
            .replace("{BLOG_URL}", &blog_url)
            .replace("{STATE_STRING}", &state),
    );
    Ok(())
}

#[handler]
async fn submit_comment(req: &mut Request, depot: &mut Depot) -> Result<(), error::Error> {
    let request_json: serde_json::Value =
        req.parse_json().await.map_err(Error::err_to_client_err)?;

    let req_state = request_json
        .get("state")
        .ok_or(error::Error::from("JSON parse error: \"state\"").into_client_err())?
        .as_str()
        .ok_or(error::Error::from("JSON parse error: \"state\"").into_client_err())?;
    let req_comment = request_json
        .get("comment_text")
        .ok_or(error::Error::from("JSON parse error: \"comment_text\"").into_client_err())?
        .as_str()
        .ok_or(error::Error::from("JSON parse error: \"comment_text\"").into_client_err())?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    sql::add_comment(&salvo_conf.db_conn_string, req_state, req_comment)?;

    let _did_remove = sql::check_remove_rng_uuid(&salvo_conf.db_conn_string, req_state)?;

    Ok(())
}

#[handler]
async fn login_to_edit_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), error::Error> {
    let salvo_conf = depot.obtain::<Config>().unwrap();
    let comment_id: String = req
        .try_query("comment_id")
        .map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(Error::err_to_client_err)?;
    let uuid = sql::create_rng_uuid(&salvo_conf.db_conn_string)?;
    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_edit_comment", salvo_conf.base_url),
        &[("comment_id", comment_id), ("blog_url", blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;
    let github_api_url = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("state", uuid.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ],
    )
    .map_err(|_| error::Error::from("Failed to parse github api url!"))?;
    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "{}";
            }}, 3000);
        "#,
        github_api_url.as_str()
    );
    res.body(format!(
        r#"<html><head><style>{}</style></head><body>
        <b>Redirecting to Github for Authentication...</b>
        <script>
        {}
        </script>
        </body></html>"#,
        COMMON_CSS, script
    ));

    Ok(())
}

#[handler]
async fn github_auth_edit_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), Error> {
    let comment_id: String = req
        .try_query("comment_id")
        .map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(Error::err_to_client_err)?;
    let state: String = req
        .try_query("state")
        .map_err(error::Error::err_to_client_err)?;
    let code: String = req
        .try_query("code")
        .map_err(error::Error::err_to_client_err)?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    let is_state_valid = sql::check_rng_uuid(&salvo_conf.db_conn_string, &state)?;
    if !is_state_valid {
        eprintln!("State is invalid (timed out?)!\n");
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request (took too long to verify)</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }

    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_edit_comment", salvo_conf.base_url),
        &[("comment_id", &comment_id)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;

    let client = reqwest::Client::builder();
    let client = client.user_agent(&salvo_conf.user_agent).build()?;
    let g_res = client
        .post("https://github.com/login/oauth/access_token")
        .query(&[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("client_secret", salvo_conf.oauth_token.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;

    let json: serde_json::Value = g_res.json().await?;
    let access_token = json.get("access_token").ok_or(error::Error::from(
        "Failed to parse access_token from response from Github!",
    ))?;
    let access_token_str: &str = access_token
        .as_str()
        .ok_or(Error::from("Github access token was not a string!"))?;
    let mut reqw_resp: Option<reqwest::Response> = None;
    for _idx in 0..3 {
        let ret = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", access_token_str))
            .header("X-Github-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(Error::from);
        if ret.is_ok() {
            let ret = ret?.error_for_status();
            if ret.is_ok() {
                reqw_resp = Some(ret?);
                break;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        } else {
            sleep(Duration::from_secs(3)).await;
        }
    }
    let user_info: serde_json::Value = reqw_resp
        .ok_or(Error::from("Failed to get user info via oauth token!"))?
        .json()
        .await?;

    let user_id: u64 = user_info
        .get("id")
        .ok_or(error::Error::from("Failed to parse user info id!"))?
        .to_string()
        .parse()?;
    let user_avatar = user_info
        .get("avatar_url")
        .ok_or(error::Error::from("Failed to parse user info avatar url!"))?
        .as_str()
        .ok_or(error::Error::from("Failed to parse user info avatar url!"))?;
    let mut user_name: Option<&serde_json::Value> = user_info.get("name");
    let user_name_str: String;

    if let Some(user_name_inner) = user_name {
        if user_name_inner.is_string() {
            user_name_str = user_name_inner
                .as_str()
                .ok_or(error::Error::from("Failed to parse user info name!"))?
                .to_owned();
        } else {
            user_name = user_info.get("login");
            user_name_str = user_name
                .ok_or(error::Error::from("User has no name or login!"))?
                .as_str()
                .ok_or(error::Error::from("Failed to parse user info login!"))?
                .to_owned();
        }
    } else {
        user_name = user_info.get("login");
        user_name_str = user_name
            .ok_or(error::Error::from("User has no name or login!"))?
            .as_str()
            .ok_or(error::Error::from("Failed to parse user info login!"))?
            .to_owned();
    }
    let user_url = user_info
        .get("html_url")
        .ok_or(error::Error::from("Failed to parse user info url!"))?
        .as_str()
        .ok_or(error::Error::from("Failed to parse user info url!"))?;

    let can_edit: bool = sql::check_edit_comment_auth(
        &salvo_conf.db_conn_string,
        &comment_id,
        &user_id.to_string(),
    )?;
    if !can_edit {
        eprintln!(
            "User tried to edit comment they didn't make! {}",
            &comment_id
        );
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request</b><br>
            <p>You are not the commentor of the comment you are trying to edit.</p>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }

    sql::add_pseudo_comment_data(
        &salvo_conf.db_conn_string,
        &state,
        user_id,
        &user_name_str,
        user_url,
        user_avatar,
        None,
        Some(&comment_id),
    )?;

    res.body(
        EDIT_COMMENT_PAGE
            .replace("{COMMON_CSS}", COMMON_CSS)
            .replace("{USER_AVATAR_URL}", user_avatar)
            .replace("{USER_NAME}", &user_name_str)
            .replace("{USER_PROFILE}", user_url)
            .replace("{BASE_URL}", &salvo_conf.base_url)
            .replace("{BLOG_URL}", &blog_url)
            .replace("{STATE_STRING}", &state)
            .replace("{COMMENT_ID}", &comment_id),
    );

    Ok(())
}

#[handler]
async fn submit_edit_comment(req: &mut Request, depot: &mut Depot) -> Result<(), Error> {
    let salvo_conf = depot.obtain::<Config>().unwrap();

    let request_json: serde_json::Value =
        req.parse_json().await.map_err(Error::err_to_client_err)?;

    let req_state = request_json
        .get("state")
        .ok_or(error::Error::from("JSON parse error: \"state\"").into_client_err())?
        .as_str()
        .ok_or(error::Error::from("JSON parse error: \"state\"").into_client_err())?;
    let req_comment = request_json
        .get("comment_text")
        .ok_or(error::Error::from("JSON parse error: \"comment_text\"").into_client_err())?
        .as_str()
        .ok_or(error::Error::from("JSON parse error: \"comment_text\"").into_client_err())?;

    sql::edit_comment(&salvo_conf.db_conn_string, req_state, req_comment)?;

    let _did_remove = sql::check_remove_rng_uuid(&salvo_conf.db_conn_string, req_state)?;

    Ok(())
}

#[handler]
async fn login_to_delete_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), Error> {
    let salvo_conf = depot.obtain::<Config>().unwrap();

    let comment_id: String = req
        .try_query("comment_id")
        .map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(Error::err_to_client_err)?;
    let uuid = sql::create_rng_uuid(&salvo_conf.db_conn_string)?;
    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_del_comment", salvo_conf.base_url),
        &[("comment_id", comment_id), ("blog_url", blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;
    let github_api_url = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("state", uuid.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ],
    )
    .map_err(|_| error::Error::from("Failed to parse github api url!"))?;
    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "{}";
            }}, 3000);
        "#,
        github_api_url.as_str()
    );
    res.body(format!(
        r#"<html><head><style>{}</style></head><body>
        <b>Redirecting to Github for Authentication...</b>
        <script>
        {}
        </script>
        </body></html>"#,
        COMMON_CSS, script
    ));

    Ok(())
}

#[handler]
async fn github_auth_del_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), Error> {
    let comment_id: String = req
        .try_query("comment_id")
        .map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_query("blog_url")
        .map_err(Error::err_to_client_err)?;
    let state: String = req
        .try_query("state")
        .map_err(error::Error::err_to_client_err)?;
    let code: String = req
        .try_query("code")
        .map_err(error::Error::err_to_client_err)?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    let is_state_valid = sql::check_rng_uuid(&salvo_conf.db_conn_string, &state)?;
    if !is_state_valid {
        eprintln!("State is invalid (timed out?)!\n");
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request (took too long to verify)</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }

    let redirect_url = Url::parse_with_params(
        &format!("{}/github_auth_del_comment", salvo_conf.base_url),
        &[("comment_id", &comment_id), ("blog_url", &blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;

    let client = reqwest::Client::builder();
    let client = client.user_agent(&salvo_conf.user_agent).build()?;
    let g_res = client
        .post("https://github.com/login/oauth/access_token")
        .query(&[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("client_secret", salvo_conf.oauth_token.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;

    let json: serde_json::Value = g_res.json().await?;
    let access_token = json.get("access_token").ok_or(error::Error::from(
        "Failed to parse access_token from response from Github!",
    ))?;
    let access_token_str: &str = access_token
        .as_str()
        .ok_or(Error::from("Github access_token was not a string!"))?;
    let mut reqw_resp: Option<reqwest::Response> = None;
    for _idx in 0..3 {
        let ret = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", access_token_str))
            .header("X-Github-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(Error::from);
        if ret.is_ok() {
            let ret = ret?.error_for_status();
            if ret.is_ok() {
                reqw_resp = Some(ret?);
                break;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        } else {
            sleep(Duration::from_secs(3)).await;
        }
    }
    let user_info: serde_json::Value = reqw_resp
        .ok_or(Error::from("Failed to get user info via oauth token!"))?
        .json()
        .await?;

    let user_id: u64 = user_info
        .get("id")
        .ok_or(error::Error::from("Failed to parse user info id!"))?
        .to_string()
        .parse()?;

    let can_del: bool = sql::check_edit_comment_auth(
        &salvo_conf.db_conn_string,
        &comment_id,
        &user_id.to_string(),
    )?;
    if !can_del {
        eprintln!(
            "User tried to delete comment they didn't make! {}",
            &comment_id
        );
        res.status_code(StatusCode::BAD_REQUEST);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Bad Request</b><br>
            <p>You are not the commentor of the comment you are trying to edit.</p>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }

    sql::try_delete_comment(&salvo_conf.db_conn_string, &comment_id, user_id)?;

    let _did_remove = sql::check_remove_rng_uuid(&salvo_conf.db_conn_string, &state)?;

    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "{}";
            }}, 5000);
        "#,
        blog_url
    );
    res.body(format!(
        r#"<html><head><style>{}</style></head><body>
        <b>Attempted Comment Delete, reloading blog url...</b>
        <script>
        {}
        </script>
        </body></html>"#,
        COMMON_CSS, script
    ));

    Ok(())
}

#[handler]
async fn get_comments_by_blog_id(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), Error> {
    let salvo_conf = depot.obtain::<Config>().unwrap();

    let blog_id: String = req.try_query("blog_id").map_err(Error::err_to_client_err)?;

    let comments = sql::get_comments_per_blog_id(&salvo_conf.db_conn_string, &blog_id)?;

    let json: String = serde_json::to_string(&comments)?;

    res.body(json);

    Ok(())
}

#[tokio::main]
async fn main() {
    let config =
        config::Config::try_from(arg_parse::Args::parse_args().unwrap().get_config_path()).unwrap();

    let salvo_conf = Config {
        db_conn_string: config.get_connection_string(),
        oauth_user: config.get_oauth_user().to_owned(),
        oauth_token: config.get_oauth_token().to_owned(),
        base_url: config.get_base_url().to_owned(),
        allowed_urls: config.get_allowed_urls().to_vec(),
        allowed_bids: config.get_allowed_bids().to_vec(),
        user_agent: config.get_user_agent().to_owned(),
    };

    sql::set_up_sql_db(&salvo_conf.db_conn_string).unwrap();

    let router = Router::new()
        .hoop(affix_state::inject(salvo_conf))
        .get(root_handler)
        .push(Router::with_path("get_comment").get(comment_text_get))
        .push(Router::with_path("get_comments").get(get_comments_by_blog_id))
        .push(Router::with_path("do_comment").get(login_to_comment))
        .push(Router::with_path("github_auth_make_comment").get(github_auth_make_comment))
        .push(Router::with_path("submit_comment").post(submit_comment))
        .push(Router::with_path("edit_comment").get(login_to_edit_comment))
        .push(Router::with_path("github_auth_edit_comment").get(github_auth_edit_comment))
        .push(Router::with_path("submit_edit_comment").post(submit_edit_comment))
        .push(Router::with_path("del_comment").get(login_to_delete_comment))
        .push(Router::with_path("github_auth_del_comment").get(github_auth_del_comment));

    let listener = TcpListener::new(format!("{}:{}", config.get_addr(), config.get_port()));

    Server::new(listener.bind().await).serve(router).await;
}
