use poem::{
    get, handler, 
    http::{header, StatusCode},
    listener::TcpListener,
    EndpointExt, Response, Result, Route, Server,
};

mod handlers; 
use handlers::UnauthorizedError;
use handlers::secrets;
use handlers::authorize;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let root = Route::new(); 

    // TODO: Checks the env and configure the routes appropriately
    let secrets_handler = secrets::Handler{};
    let root = secrets_handler.install(root);

    let authorize_handler = authorize::Handler{};
    let root = authorize_handler.install(root);

    // Enable TLS
    // let key = fs::read_to_string(format!("{}/localhost-key.pem", home_dir));
    // let cert = fs::read_to_string(format!("{}/localhost.pem", home_dir));

    // Server::new(TcpListener::bind("0.0.0.0:8443")
    //     .rustls(RustlsConfig::new().key(key).cert(cert)))
    //     .run(app)
    //     .await

    let app = root.catch_error(|err: UnauthorizedError| async move {
        eprintln!("{:?}", err); 
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::WWW_AUTHENTICATE, r#"Bearer authorization="https://localhost:8443/authorize", resource="https://vault.azure.net""#)
            .finish()
    });

    let listener = TcpListener::bind("0.0.0.0:8443");

    Server::new(listener).run(app).await
}
