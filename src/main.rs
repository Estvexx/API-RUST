use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::env;

use axum_keycloak_auth::{
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
    PassthroughMode,
    Url,
};

#[derive(Deserialize)]
struct UserPayload {
    name: String,
    email: String,
}

#[derive(Serialize, FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().connect(&db_url).await.expect("Failed to connect to DB");
    sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    let keycloak_instance = KeycloakAuthInstance::new(
        KeycloakConfig::builder()
            .server(Url::parse("http://host.docker.internal:8080/").unwrap())
            .realm("my-api-realm".to_string())
            .build(),
    );

    let protected_route = Router::new()
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/protected", get(rota_protegida))
        .layer(
            KeycloakAuthLayer::<String>::builder()
                .instance(keycloak_instance)
                .passthrough_mode(PassthroughMode::Block)
                .expected_audiences(vec!["account".to_string()])
                .build(),
        );

    let app = Router::new()
        .route("/", get(root))
        .merge(protected_route) // Juntar a rota protegida ao resto
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    println!("🚀 Server running on port 8001");
    axum::serve(listener, app).await.unwrap();
}

async fn rota_protegida() -> &'static str {
    "🔓 Acesso autorizado! O teu token do Keycloak é válido."
}

//Endpoint Handlers
async fn root() -> &'static str {
    "Welcome to the User Management API!"
}

//GET ALL
async fn list_users(State(pool): State<PgPool>) -> Result<Json<Vec<User>>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//CREATE USER
async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<UserPayload>
    ) -> Result<(StatusCode, Json<User>), StatusCode> {
    sqlx::query_as::<_, User>("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *")
        .bind(payload.name)
        .bind(payload.email)
        .fetch_one(&pool).await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//GET USER BY ID
async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<i32>
    ) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool).await
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

//UPDATE USER
async fn update_user(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<UserPayload>
    ) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>("UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *")
        .bind(payload.name)
        .bind(payload.email)
        .bind(id)
        .fetch_one(&pool).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//DELETE USER
async fn delete_user(
    State(pool): State<PgPool>,
    Path(id): Path<i32>
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
