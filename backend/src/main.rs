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
mod helper;
mod sql;

use error::Error;
use reqwest::Url;
use salvo::prelude::*;
use sql::set_up_sql_db;

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
        <img width="64" height="64" src="{USER_AVATAR_URL}" /> <b>{USER_NAME}</b> <a href="{USER_PROFILE}">(User Profile)</a><br>
        <textarea id="comment_text" name="comment_text" rows="10" autofocus=true><br>
        <button id="comment_submit_button">Submit</button>
        <script>
            "use strict;"

            async function submit_comment(json) {
                const response = await fetch("{BASE_URL}/submit_comment",
                    {
                        method: "POST",
                        body: json,
                    }
                );
                if (!response.ok) {
                    throw new Error(`Response status: ${response.status}`);
                } else {
                    window.location = "{BLOG_URL}";
                }
            }

            window.addEventListener("load", (event) => {
                let button = getElementById("comment_submit_button");
                let textarea = getElementById("comment_text");

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
}

#[handler]
async fn root_handler(res: &mut Response) {
    res.body(format!(
        "<html><head><style>{}</style></head><body><h1>Welcome</h1></body></html>",
        COMMON_CSS
    ));
}

#[handler]
async fn login_to_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), error::Error> {
    let blog_id: String = req.try_param("blog_id").map_err(Error::err_to_client_err)?;
    let blog_url: String = req
        .try_param("blog_url")
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
        &format!("{}/github_authenticated", salvo_conf.base_url),
        &[("blog_id", blog_id), ("blog_url", blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;
    let redirect_url_escaped = helper::percent_escape_uri(redirect_url.as_str());
    let github_api_url = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("state", uuid.as_str()),
            ("redirect_url", redirect_url_escaped.as_str()),
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
async fn github_authenticated(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), error::Error> {
    let blog_id: String = req
        .try_param("blog_id")
        .map_err(error::Error::err_to_client_err)?;
    let blog_url: String = req
        .try_param("blog_url")
        .map_err(error::Error::err_to_client_err)?;
    let state: String = req
        .try_param("state")
        .map_err(error::Error::err_to_client_err)?;
    let code: String = req
        .try_param("code")
        .map_err(error::Error::err_to_client_err)?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    let redirect_url = Url::parse_with_params(
        &format!("{}/github_authenticated", salvo_conf.base_url),
        &[("blog_id", &blog_id), ("blog_url", &blog_url)],
    )
    .map_err(|_| error::Error::from("Failed to parse redirect url!"))?;
    let redirect_url_escaped = helper::percent_escape_uri(redirect_url.as_str());

    let client = reqwest::Client::new();
    let g_res = client
        .post("https://github.com/login/oauth/access_token")
        .query(&[
            ("client_id", salvo_conf.oauth_user.as_str()),
            ("client_secret", salvo_conf.oauth_token.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_url_escaped.as_str()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;

    let json: serde_json::Value = g_res.json().await?;
    let access_token = json.get("access_token").ok_or(error::Error::from(
        "Failed to parse access_token from response from Github!",
    ))?;
    if !access_token.is_string() {
        eprintln!("Received access_token is not a string!\n");
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.body(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Internal Server Error</b>
            </body></html>"#,
            COMMON_CSS,
        ));
        return Ok(());
    }
    let access_token: String = access_token.to_string();
    let user_info = client
        .get("https://api.github.com/user")
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Github-Api-Version", "2022-11-28")
        .send()
        .await?;
    let user_info: serde_json::Value = user_info.json().await?;

    let user_id: u64 = user_info
        .get("id")
        .ok_or(error::Error::from("Failed to parse user info id!"))?
        .to_string()
        .parse()?;

    let user_name: String = user_info
        .get("name")
        .ok_or(error::Error::from("Failed to parse user info name!"))?
        .to_string();

    let user_url: String = user_info
        .get("html_url")
        .ok_or(error::Error::from("Failed to parse user info profile url!"))?
        .to_string();

    let user_avatar_url: String = user_info
        .get("avatar_url")
        .ok_or(error::Error::from(
            "Failed to parse user info profile avatar url!",
        ))?
        .to_string();

    sql::add_pseudo_comment_data(
        &salvo_conf.db_conn_string,
        &state,
        user_id,
        &user_name,
        &user_url,
        &user_avatar_url,
        &blog_id,
    )?;

    res.body(
        WRITE_COMMENT_PAGE
            .replace("{BLOG_ID}", &blog_id)
            .replace("{COMMON_CSS}", COMMON_CSS)
            .replace("{USER_AVATAR_URL}", &user_avatar_url)
            .replace("{USER_NAME}", &user_name)
            .replace("{USER_PROFILE}", &user_url)
            .replace("{BASE_URL}", &salvo_conf.base_url)
            .replace("{BLOG_URL}", &blog_url)
            .replace("{STATE_STRING}", &state),
    );
    Ok(())
}

#[handler]
async fn submit_comment(req: &mut Request, depot: &mut Depot) -> Result<(), error::Error> {
    let request_json: serde_json::Value = req.parse_json().await?;

    let req_state: String = request_json
        .get("state")
        .ok_or(error::Error::from("JSON parse error: \"state\"").into_client_err())?
        .to_string();
    let req_comment: String = request_json
        .get("comment_text")
        .ok_or(error::Error::from("JSON parse error: \"comment_text\"").into_client_err())?
        .to_string();

    let salvo_conf = depot.obtain::<Config>().unwrap();

    sql::add_comment(&salvo_conf.db_conn_string, &req_state, &req_comment)?;

    let _did_remove = sql::check_remove_rng_uuid(&salvo_conf.db_conn_string, &req_state)?;

    Ok(())
}

// TODO: get_comment

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
    };

    set_up_sql_db(&salvo_conf.db_conn_string).unwrap();

    let router = Router::new()
        .hoop(affix_state::inject(salvo_conf))
        .get(root_handler)
        .push(Router::with_path("do_comment").get(login_to_comment))
        .push(Router::with_path("github_authenticated").get(github_authenticated))
        .push(Router::with_path("submit_comment").post(submit_comment));

    let listener = TcpListener::new(format!("{}:{}", config.get_addr(), config.get_port()));

    Server::new(listener.bind().await).serve(router).await;
}
