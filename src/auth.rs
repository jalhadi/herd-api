use serde::{Serialize};
use serde_json::json;
use actix_web::{error, http, Error, HttpRequest, client};
use actix_web::http::header::Header;
use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use actix_web_httpauth::extractors::basic::BasicAuth;

#[derive(Debug, Serialize)]
pub struct AuthStruct {
    pub account_id: String,
    pub api_key: String,
}

pub fn get_auth_struct(r: &HttpRequest) -> Result<AuthStruct, Error> {
    let auth = Authorization::<Basic>::parse(r)?.into_scheme();
    let account_id = auth.user_id().to_string();
    Ok(AuthStruct {
        account_id: account_id.clone(),
        api_key: auth.password().unwrap().to_string()
    })
}

pub async fn authenticate_websocket_connection(r: &HttpRequest) -> Result<String, Error> {
    let auth_struct = get_auth_struct(r)?;

    let api_validation = json!({
        "api_authentication": auth_struct,
    });
    let client = client::Client::default();
    let response = client
        .put("http://localhost:3000/api_key/validate")
        .header("User-Agent", "Actix-web")
        .send_json(&api_validation)
        .await?;
    if response.status() != http::StatusCode::OK {
        return Err(error::ErrorUnauthorized("Unauthorized"));
    }
    Ok(auth_struct.account_id)
}

pub async fn authenticate_connection(credentials: BasicAuth) -> Result<String, Error> {
    let account_id = credentials.user_id().to_string();
    let password = credentials.password().unwrap().to_string();
    let auth_struct = AuthStruct {
        account_id: account_id,
        api_key: password
    };
    let api_validation = json!({
        "api_authentication": auth_struct,
    });
    let client = client::Client::default();
    let response = client
        .put("http://localhost:3000/api_key/validate")
        .header("User-Agent", "Actix-web")
        .send_json(&api_validation)
        .await?;
    if response.status() != http::StatusCode::OK {
        return Err(error::ErrorUnauthorized("Unauthorized"));
    }
    Ok(auth_struct.account_id)
}
