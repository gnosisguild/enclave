use jwt::SignWithKey;
use sha2::Sha256;
use std::collections::BTreeMap;
use hmac::{Hmac, Mac};
use log::info;

use actix_web::{web, HttpResponse, Responder};

use crate::server::models::{AppState, AuthenticationLogin, AuthenticationDB, AuthenticationResponse};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/auth")
                .route("/login", web::post().to(authenticate_login))
        );
}

/// Authenticate a login
/// 
/// # Arguments
/// 
/// * `state` - The application state
/// * `AuthenticationLogin` - The post ID for the login
/// 
/// # Returns
/// 
/// * `AuthenticationResponse` - The response indicating the success or failure of the login
async fn authenticate_login(state: web::Data<AppState>, data: web::Json<AuthenticationLogin>) -> impl Responder {
    let incoming = data.into_inner();
    info!("Twitter Login Request");

    // Generate HMAC token
    let hmac_key: Hmac<Sha256> = Hmac::new_from_slice(b"some-secret").unwrap();
    let mut claims = BTreeMap::new();
    claims.insert("postId", incoming.postId);
    let token = claims.sign_with_key(&hmac_key).unwrap();

    let key = "authentication";
    let db = &state.sled.db.write().await;
    let mut is_new = false; // Track if it's a new login

    // Perform DB update and fetch the current state
    db.update_and_fetch(key, |old| {
        let mut auth_db = old
            .map(|existing| serde_json::from_slice::<AuthenticationDB>(&existing).unwrap())
            .unwrap_or_else(|| AuthenticationDB { jwt_tokens: Vec::new() });

        // Check if the token is new
        if !auth_db.jwt_tokens.contains(&token) {
            auth_db.jwt_tokens.push(token.clone());
            is_new = true;  // Mark as a new token
        }

        // Serialize the updated auth_db back into bytes
        Some(serde_json::to_vec(&auth_db).unwrap())
    }).unwrap();

    let (response_text, log_message) = if is_new {
        info!("Inserting new login to db.");
        ("Authorized", "New login inserted.")
    } else {
        info!("Found previous login.");
        ("Already Authorized", "Previous login found.")
    };

    info!("{}", log_message);

    HttpResponse::Ok().json(AuthenticationResponse {
        response: response_text.to_string(),
        jwt_token: token,
    })
}
