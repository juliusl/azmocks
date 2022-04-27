use poem::{
    async_trait, get, handler, put, web::{Json, Path}, FromRequest, Request, RequestBody, Route,
    RouteMethod,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use logos::Logos;

use super::UnauthorizedError;

/// Handler provides some utility methods for
/// adding routes to a route object
pub struct Handler {
    enable_secret_interpreter: bool
}

impl Handler {
    pub fn new(interpret_secret_names: bool) -> Self {
        Self {
            enable_secret_interpreter: interpret_secret_names
        }
    }

    /// install adds routes for secret handling to the route object
    pub fn install(&self, route: Route) -> Route {
        route
            .at("/secrets", self.get())
            .at("/secrets/:secretname/", self.get())
            .at("/secrets/:secretname/:version", self.get())
            .at("/secrets/:secretname", self.put())
    }

    /// returns the route method to get secrets
    fn get(&self) -> RouteMethod {
        if self.enable_secret_interpreter {
            get(get_secret_and_interpret)
        } else {
            get(get_secret_test)
        }
    }

    /// returns the route method to put secrets
    fn put(&self) -> RouteMethod {
        put(put_secret_test)
    }
}

#[handler]
fn get_secret_and_interpret(Path(secretname): Path<String>, Path(version): Path<String>) -> Json<Secret> {
    // Generally the secret name will include the type of secret
    // There a couple of common patterns here
    let secret_value = MockServiceSecret::interpret(&secretname);

    // TODO -- generate an id
    Json(Secret {
        id: Some("test-id".to_string()),
        value: secret_value.to_string(),
        content_type: Some("text/string".to_string()),
        attributes: Some(SecretAttributes {
            id: "".to_string(),
            enabled: true,
            exp: -1,
            nbf: -1,
        }),
        tags: None,
    })
}


/// get_secret_test returns the same test secret, used to spot check the server can be reached
#[handler]
#[instrument(level = "debug")]
fn get_secret_test(req: &Request) -> Json<Secret> {
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

/// put_secret_test returns the same test secret, used to spot check the server can be reached
#[handler]
#[instrument(level = "debug")]
async fn put_secret_test(secret: Secret) -> Json<Secret> {
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

#[derive(Default, Serialize, Deserialize, Debug)]
struct Secret {
    id: Option<String>,
    value: String,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    attributes: Option<SecretAttributes>,
    tags: Option<serde_json::value::Value>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
struct SecretAttributes {
    id: String,
    enabled: bool,
    exp: i32,
    nbf: i32,
}

#[async_trait]
impl<'a> FromRequest<'a> for Secret {
    /// from_request handles the KV client secret flow which is looking for the auth method
    /// the beginning of the sequence the client sends an empty body
    async fn from_request(_: &'a Request, body: &mut RequestBody) -> poem::Result<Secret> {
        match body.take() {
            Ok(r) => {
                match r.into_json::<Secret>().await {
                    Ok(s) => Ok(s),
                    // This happens if the body is empty, in this case we need to send a ChallengeHeader back to the client
                    // keyvault client will not send the body of the request until it knows what type of authorization to use
                    Err(_) => Err(poem::Error::from(UnauthorizedError {})),
                }
            }
            _ => Err(poem::Error::from(UnauthorizedError {})),
        }
    }
}

#[derive(Logos, Debug, PartialEq)]
enum MockServiceSecret {
    // Tokens can be literal strings, of any length.
    #[token("StorageConnectionString")]
    StorageConnectionString,

    #[token("Blob")]
    Blob,

    #[token("Queue")]
    Queue,

    #[token("Table")]
    Table,

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl MockServiceSecret {
    /// interpret secret_name pattern into the corresponding mock secret value
    /// generally the secret value is a connnection string with all of the endpoint parameters
    fn interpret(secret_name: &str) -> String {
        let mut lexer = MockServiceSecret::lexer(secret_name);

        let mut is_blob = false;
        let mut is_queue = false;
        let mut is_table = false;
        let mut is_storage_connection_string = false;
    
        // parse the secret name
        loop {
            match lexer.next() {
                Some(MockServiceSecret::Blob) => is_blob = true,
                Some(MockServiceSecret::Queue) => is_queue = true,
                Some(MockServiceSecret::Table) => is_table = true,
                Some(MockServiceSecret::StorageConnectionString) => is_storage_connection_string = true,
                Some(MockServiceSecret::Error) => break,
                None => break,
            }
        }
    
        // interpret into some secret value
        let secret_value = match is_storage_connection_string {
            true => match (is_blob, is_queue, is_table) {
                (true, false, false) => "UseDevelopmentStorage=true;BlobEndpoint=http://azurite-blob.azmocks.cluster.local/devstoreaccount1;",
                (false, true, false) => "UseDevelopmentStorage=true;QueueEndpoint=http://azurite-queue.azmocks.cluster.local/devstoreaccount1;",
                (false, false, true) => "UseDevelopmentStorage=true;TableEndpoint=http://azurite-table.azmocks.cluster.local/devstoreaccount1;",
                _ => "UseDevelopmentStorage=true;BlobEndpoint=http://azurite-blob.azmocks.cluster.local/devstoreaccount1;QueueEndpoint=http://azurite-queue.azmocks.cluster.local/devstoreaccount1;TableEndpoint=http://azurite-table.azmocks.cluster.local/devstoreaccount1;",
            },
            false => ""
        };

        secret_value.to_string()
    }
}
