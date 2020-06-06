use diesel::prelude::*;
use openssl::symm::{encrypt, Cipher};
use data_encoding::{HEXUPPER, HEXLOWER};
use actix_web::{error, Error};
use actix_web_httpauth::extractors::basic::BasicAuth;
use diesel::pg::PgConnection;
use openssl::memcmp;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;

use crate::models;

fn validate_api_key<'a>(
    account_id: &'a str,
    api_key: &'a str,
    api_cipher_key: &'a str,
    conn: &PgConnection,
) -> bool {
    use crate::schema::accounts::dsl;

    let result = dsl::accounts
        .filter(dsl::id.eq(account_id))
        .first::<models::Account>(conn)
        .expect("Error getting account information.");

    let cipher = Cipher::aes_256_cbc();
    let iv = &HEXUPPER.decode(result.cipher_iv.as_bytes()).unwrap();
    let ciphertext = encrypt(
        cipher,
        &HEXLOWER.decode(api_cipher_key.as_bytes()).unwrap(),
        Some(&iv),
        &api_key.as_bytes()
    ).expect("Error generating ciphertext");

    if HEXUPPER.encode(&ciphertext) == result.secret_key {
        return true;
    }
    return false;
}

pub fn authenticate_connection<'a>(auth: BasicAuth, api_cipher_key: &'a str, conn: &PgConnection) -> Result<String, Error> {
    match validate_api_key(&auth.user_id(), &auth.password().unwrap(), api_cipher_key, conn) {
        true => Ok(auth.user_id().to_string()),
        false => Err(error::ErrorUnauthorized("Unauthorized"))
    }
}

pub fn verify_hmac_sigature<'a>(
    url_path: &'a str,
    account_id: &'a str,
    time: &'a str,
    signature: &'a str,
    hmac_key: &'a str,
) -> bool {
    // TODO: STORE SOMEWHERE
    let key = PKey::hmac(hmac_key.as_bytes()).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(url_path.as_bytes()).unwrap();
    signer.update(account_id.as_bytes()).unwrap();
    signer.update(time.as_bytes()).unwrap();
    let hmac = signer.sign_to_vec().unwrap();

    memcmp::eq(&hmac, &HEXLOWER.decode(signature.as_bytes()).unwrap())
}
