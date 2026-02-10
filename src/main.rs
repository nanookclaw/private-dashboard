#[macro_use] extern crate rocket;

mod db;
mod models;
mod auth;
mod routes;

use std::path::PathBuf;
use std::sync::Arc;
use db::Db;
use rocket::fs::{FileServer, Options};
use rocket_cors::{AllowedOrigins, CorsOptions};

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "dashboard.db".into());
    let database = Arc::new(Db::new(&db_path).expect("Failed to initialize database"));

    // Generate or retrieve manage key
    let manage_key = match database.get_manage_key() {
        Some(key) => {
            println!("🔑 Manage key: {}", key);
            key
        }
        None => {
            let key = format!("dash_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
            database.set_manage_key(&key);
            println!("🔑 Generated new manage key: {}", key);
            key
        }
    };

    let _ = manage_key; // Used for display only; auth checks DB directly

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .to_cors()
        .expect("CORS configuration failed");

    let static_dir: PathBuf = std::env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../frontend/dist"));

    let mut build = rocket::build()
        .attach(cors)
        .manage(database)
        .mount("/api/v1", routes![
            routes::health,
            routes::submit_stats,
            routes::get_stats,
            routes::get_stat_history,
        ])
        .mount("/", routes![
            routes::llms_txt,
            routes::openapi_spec,
        ]);

    if static_dir.exists() {
        build = build.mount("/", FileServer::new(static_dir, Options::Index));
    }

    build
}
