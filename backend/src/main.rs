mod arg_parse;
mod config;
mod error;
mod sql;

use salvo::{Result, prelude::*};
use sql::set_up_sql_db;

#[derive(Default, Clone, Debug)]
struct Config {
    db_conn_string: String,
    oauth_user: String,
    oauth_token: String,
    base_url: String,
}

fn set_up_db(db_conn_str: &str) -> std::result::Result<(), error::Error> {
    set_up_sql_db(db_conn_str)
}

fn get_common_css() -> &'static str {
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
async fn login_to_comment(res: &mut Response, depot: &mut Depot) -> String {
    let salvo_conf = depot.obtain::<Config>().unwrap();
    let script = format!(
        r#"
            "use strict;"
            setTimeout(() => {{
                window.location = "https://github.com/login/oauth/authorize?{}";
            }}, 3000);
        "#,
        format!("client_id={}&scope=read:user&", salvo_conf.oauth_user)
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

    set_up_db(&salvo_conf.db_conn_string).unwrap();

    let router = Router::new()
        .hoop(affix_state::inject(salvo_conf))
        .get(root_handler)
        .push(Router::with_path("do_comment").get(login_to_comment));

    let listener = TcpListener::new(format!("{}:{}", config.get_addr(), config.get_port()));

    Server::new(listener.bind().await).serve(router).await;
}
