use serde::{Deserialize};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use data_encoding::{HEXUPPER, HEXLOWER};
use ring::rand::SecureRandom;
use openssl::symm::{encrypt, decrypt, Cipher};
use serde::Serialize;
use rand::Rng; 
use rand::distributions::Alphanumeric;

use crate::models;

#[derive(Deserialize, Debug)]
pub struct SignupParams{
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

pub fn create_account<'a>(
    account_id: &'a str,
    api_cipher_key: &'a str,
    conn: &PgConnection,
) -> Result<(), &'static str> {
    let rng = ring::rand::SystemRandom::new();

    let api_key: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .collect();

    let mut iv = [0u8; 16];
    rng.fill(&mut iv).expect("Error generating iv.");

    let cipher = Cipher::aes_256_cbc();
    let ciphertext = encrypt(
        cipher,
        &HEXLOWER.decode(api_cipher_key.as_bytes()).unwrap(),
        Some(&iv),
        &api_key.as_bytes()
    ).expect("Error generating ciphertext");

    use crate::schema::accounts;

    let new_api_key = models::NewAccount {
        id: account_id,
        secret_key: &HEXUPPER.encode(&ciphertext),
        cipher_iv: &HEXUPPER.encode(&iv),
    };

    let result = diesel::insert_into(accounts::table)
        .values(&new_api_key)
        .execute(conn);

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err("Error creating account.")
    }
}

#[derive(Serialize)]
pub struct ApiKeyResult {
    pub api_key: String,
}

pub fn get_api_key<'a>(
    account_id: &'a str,
    api_cipher_key: &'a str,
    conn: &PgConnection,
) -> Result<ApiKeyResult, diesel::result::Error> {
    use crate::schema::accounts::dsl;

    let result = dsl::accounts
        .filter(dsl::id.eq(account_id))
        .first::<models::Account>(conn)?;

    let cipher = Cipher::aes_256_cbc();
    let data = &HEXUPPER.decode(result.secret_key.as_bytes()).unwrap();
    let iv = &HEXUPPER.decode(result.cipher_iv.as_bytes()).unwrap();
    let decrypted_key = decrypt(
        cipher,
        &HEXLOWER.decode(api_cipher_key.as_bytes()).unwrap(),
        Some(iv),
        data
    ).unwrap();
    let api_key = String::from_utf8(decrypted_key).unwrap();
    Ok(ApiKeyResult { api_key })
}
