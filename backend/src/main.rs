mod arg_parse;
mod config;
mod error;
mod sql;

use salvo::prelude::*;
use sql::set_up_sql_db;

#[derive(Default, Clone, Debug)]
struct Config {
    db_conn_string: String,
    oauth_user: String,
    oauth_token: String,
    base_url: String,
}

pub fn get_common_css() -> &'static str {
    "body {
        color: #FFF;
        background-color: #000;
    }
    a {
        color: #8F8;
    }"
}

#[handler]
async fn root_handler() -> &'static str {
    "<html><body><b>test</b></body></html>"
}

#[handler]
async fn login_to_comment(req: &mut Request, res: &mut Response, depot: &mut Depot) -> String {
    let blog_id = req.params_mut().get("blog_id");
    let mut blog_str = String::new();
    if let Some(blog_id_s) = blog_id {
        blog_str = format!("/{}", blog_id_s);
    }
    let salvo_conf = depot.obtain::<Config>().unwrap();
    let uuid = sql::create_rng_uuid(&salvo_conf.db_conn_string);
    if uuid.is_err() {
        eprintln!("Failed to generate uuid!");
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Internal Server Error</b>
            </body></html>"#,
            get_common_css(),
        );
    }
    let uuid = uuid.unwrap();
    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "https://github.com/login/oauth/authorize?{}";
            }}, 3000);
        "#,
        format!(
            "client_id={}&state={}&redirect_url={}/github_authenticated{}",
            salvo_conf.oauth_user, uuid, salvo_conf.base_url, blog_str
        )
    );
    format!(
        r#"<html><head><style>{}</style></head><body>
        <b>Redirecting to Github for Authentication...</b>
        <script>
        {}
        </script>
        </body></html>"#,
        get_common_css(),
        script
    )
}

#[handler]
async fn github_authenticated(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<String, error::Error> {
}

#[handler]
async fn github_authenticated_blog_id(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<String, error::Error> {
    let blog_id: Option<String> = req.param("bid");
    if blog_id.is_none() {
        return github_authenticated::github_authenticated(req, res, depot).await;
    }

    let blog_id = blog_id.unwrap();

    let state: String = req.param("state").ok_or(error::Error::from(
        "Redirect from Github had invalid state!",
    ))?;

    let code: String = req.param("code").ok_or(error::Error::from(
        "Redirect from Github had no temporary code/token!",
    ))?;

    let salvo_conf = depot.obtain::<Config>().unwrap();

    let client = reqwest::Client::new();
    let g_res = client
        .post("https://github.com/login/oauth/access_token")
        .query(&[
            ("client_id", &salvo_conf.oauth_user),
            ("client_secret", &salvo_conf.oauth_token),
            ("code", &code),
            (
                "redirect_uri",
                &format!("{}/github_authenticated/{}", salvo_conf.base_url, &blog_id),
            ),
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
        return Ok(format!(
            r#"<html><head><style>{}</style></head><body>
            <b>Internal Server Error</b>
            </body></html>"#,
            get_common_css(),
        ));
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
    )?;

    // TODO: Redirect to write comment page.

    Err(error::Error::from("Unimplemented"))
}

#[handler]
async fn write_comment(res: &mut Response, depot: &mut Depot) -> Result<String, error::Error> {}

#[tokio::main]
async fn main() {
    let config =
        config::Config::try_from(arg_parse::Args::parse_args().unwrap().get_config_path()).unwrap();

    let salvo_conf = Config {
        db_conn_string: config.get_connection_string(),
        oauth_user: config.get_oauth_user().to_owned(),
        oauth_token: config.get_oauth_token().to_owned(),
        base_url: config.get_base_url().to_owned(),
    };

    set_up_sql_db(&salvo_conf.db_conn_string).unwrap();

    let router = Router::new()
        .hoop(affix_state::inject(salvo_conf))
        .get(root_handler)
        .push(Router::with_path("do_comment").get(login_to_comment))
        .push(
            Router::with_path("github_authenticated")
                .get(github_authenticated)
                .push(Router::with_path("{bid}").get(github_authenticated_blog_id)),
        );

    let listener = TcpListener::new(format!("{}:{}", config.get_addr(), config.get_port()));

    Server::new(listener.bind().await).serve(router).await;
}
