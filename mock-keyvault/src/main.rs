use poem::{
    async_trait,
    error::ResponseError,
    get, handler,
    http::{header, StatusCode},
    listener::{Listener, RustlsConfig, TcpListener},
    put,
    web::Json,
    EndpointExt, FromRequest, Request, RequestBody, Response, Result, Route, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt::Display, fs};

#[derive(Default, Serialize, Deserialize, Debug)]
struct Secret {
    id: Option<String>,
    value: String,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    attributes: Option<SecretAttributes>,
    tags: Option<serde_json::value::Value>,
}

#[async_trait]
impl<'a> FromRequest<'a> for Secret {
    async fn from_request(_: &'a Request, body: &mut RequestBody) -> poem::Result<Secret> {
        match body.take() {
            Ok(r) => {
                match r.into_json::<Secret>().await {
                    Ok(s) => Ok(s),
                    // This happens if the body is empty, in this case we need to send a ChallengeHeader back to the client
                    // keyvault client will not send the body of the request until it knows what type of authorization to use
                    Err(_) => Err(poem::Error::from(UnauthorizedError {})),
                }
            },
            _ => Err(poem::Error::from(UnauthorizedError {}))
        }
    }
}

#[derive(Debug, thiserror::Error)]
struct UnauthorizedError;

impl Display for UnauthorizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "unauthorized")
    }
}

impl ResponseError for UnauthorizedError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
struct SecretAttributes {
    id: String,
    enabled: bool,
    exp: i32,
    nbf: i32,
}

#[handler]
fn get_secrets_handler(req: &Request) -> Json<Secret> {
    println!("{:?}", req);

    return Json(Secret {
        id: Some("test-id".to_string()),
        value: "test-secret".to_string(),
        content_type: Some("test.secret/string".to_string()),
        attributes: Some(SecretAttributes {
            id: "".to_string(),
            enabled: true,
            exp: -1,
            nbf: -1,
        }),
        tags: Some(json!({})),
    });
}

#[handler]
async fn put_secrets_handler(secret: Secret) -> Json<Secret> {
    println!("{:?}", secret);

    Json(Secret {
        id: Some("https://localhost:8443/secrets/test-secret/test-version".to_string()),
        value: "test-secret".to_string(),
        content_type: Some("test.secret/string".to_string()),
        attributes: Some(SecretAttributes {
            id: "https://localhost:8443/secrets/test-secret/test-version".to_string(),
            enabled: true,
            exp: -1,
            nbf: -1,
        }),
        tags: Some(json!({})),
    })
}

#[handler]
async fn get_token_handler() -> Result<&'static str> {
    println!("called get token handler");
    Ok("ok")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let home_dir = std::env::var("HOME").unwrap();

    let app = Route::new()
        .at("/", get(get_secrets_handler))
        .at("/secrets", get(get_secrets_handler))
        .at("/secrets/:secretname", put(put_secrets_handler))
        .at("/secrets/:secretname/", get(get_secrets_handler))
        .at("/secrets/:secretname/:version", get(get_secrets_handler))
        .at("/authorize", get(get_token_handler))
        .catch_error(|err: UnauthorizedError| async move {
            eprintln!("{:?}", err); 
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::WWW_AUTHENTICATE, r#"Bearer authorization="https://localhost:8443/authorize", resource="https://vault.azure.net""#)
                .finish()
        });

    // Enable TLS
    // TODO: Since this is a K8's service, it should be mounted in secrets
    // TODO: use mkcert -- 
    // let key = fs::read_to_string(format!("{}/localhost-key.pem", home_dir));
    // let cert = fs::read_to_string(format!("{}/localhost.pem", home_dir));

    // Server::new(TcpListener::bind("0.0.0.0:8443")
    //     .rustls(RustlsConfig::new().key(key).cert(cert)))
    //     .run(app)
    //     .await


    Server::new(TcpListener::bind("0.0.0.0:8443"))
        .run(app)
        .await
}
