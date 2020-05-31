use serde::{Deserialize};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use data_encoding::HEXUPPER;
use ring::rand::SecureRandom;
use openssl::symm::{encrypt, decrypt, Cipher};
use serde::Serialize;
use rand::Rng; 
use rand::distributions::Alphanumeric;

use crate::models;

// 256 bit string
// TODO: generate a key and store somewhere
pub const TEMP_KEY: &'static [u8; 32] = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";

#[derive(Deserialize, Debug)]
pub struct SignupParams{
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

pub fn create_account<'a>(account_id: &'a str, conn: &PgConnection) -> Result<(), &'static str> {
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
        TEMP_KEY,
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

pub fn get_api_key<'a>(account_id: &'a str, conn: &PgConnection) -> Result<ApiKeyResult, diesel::result::Error> {
    use crate::schema::accounts::dsl;

    let result = dsl::accounts
        .filter(dsl::id.eq(account_id))
        .first::<models::Account>(conn)?;

    let cipher = Cipher::aes_256_cbc();
    let data = &HEXUPPER.decode(result.secret_key.as_bytes()).unwrap();
    let iv = &HEXUPPER.decode(result.cipher_iv.as_bytes()).unwrap();
    let decrypted_key = decrypt(
        cipher,
        TEMP_KEY,
        Some(iv),
        data
    ).unwrap();
    let api_key = String::from_utf8(decrypted_key).unwrap();
    Ok(ApiKeyResult { api_key })
}
