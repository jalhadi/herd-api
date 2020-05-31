use diesel::prelude::*;
use openssl::symm::{encrypt, Cipher};
use data_encoding::HEXUPPER;
use actix_web::{error, Error};
use actix_web_httpauth::extractors::basic::BasicAuth;
use diesel::pg::PgConnection;

use crate::account::{TEMP_KEY};
use crate::models;

fn validate_api_key<'a>(account_id: &'a str, api_key: &'a str, conn: &PgConnection) -> bool {
    use crate::schema::accounts::dsl;

    let result = dsl::accounts
        .filter(dsl::id.eq(account_id))
        .first::<models::Account>(conn)
        .expect("Error getting account information.");

    let cipher = Cipher::aes_256_cbc();
    let iv = &HEXUPPER.decode(result.cipher_iv.as_bytes()).unwrap();
    let ciphertext = encrypt(
        cipher,
        TEMP_KEY,
        Some(&iv),
        &api_key.as_bytes()
    ).expect("Error generating ciphertext");

    if HEXUPPER.encode(&ciphertext) == result.secret_key {
        return true;
    }
    return false;
}

pub fn authenticate_connection(auth: BasicAuth, conn: &PgConnection) -> Result<String, Error> {
    match validate_api_key(&auth.user_id(), &auth.password().unwrap(), conn) {
        true => Ok(auth.user_id().to_string()),
        false => Err(error::ErrorUnauthorized("Unauthorized"))
    }
}
