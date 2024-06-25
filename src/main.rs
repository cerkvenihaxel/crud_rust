use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router
};

use serde::{Deserialize, Serialize};
use serde_json::json;
//use serde_json::Value::String;
use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres};
use sqlx::postgres::PgSeverity::Error;
use tokio::net::TcpListener;

#[tokio::main]

async fn main() {
    // expose the environment variables

    dotenvy::dotenv().expect("Unable to access .env file");

    // set variables from the environment variables

    let server_address = std::env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:3000".to_owned());
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not found in the env file");

    // create the database pool

    let db_pool: Pool<Postgres> = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
        .expect("Can't connect to the database");

    // create our TCP listener

    let listener = TcpListener::bind(server_address).await.expect("Could not create TCP List");

    println!("Listening on {}", listener.local_addr().unwrap());

    // compose the routes

    let app: Router = Router::new()
                        .route("/", get( || async { "Hello World" }))
                        .route("/tasks", get(get_tasks).post(create_task))
                        .route("/tasks/:task_id", patch(update_tasks).delete(delete_task))
                        .with_state(db_pool);


    // serve the application

    axum::serve(listener, app)
        .await
        .expect("Error serving application");
}

#[derive(Serialize)]

struct TaskRow {
    task_id: i32,
    name: String,
    priority: Option<i32>
}
async fn get_tasks(
    State(db_pool): State<PgPool>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let rows: Vec<TaskRow> = sqlx::query_as!(TaskRow, "SELECT * FROM tasks ORDER BY task_id")
        .fetch_all(&db_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "data": rows}).to_string(),
    ))
}

#[derive(Deserialize)]

struct CreateTaskReq {
    name: String,
    priority: Option<i32>
}

#[derive(Serialize)]

struct CreateTaskRow {
    task_id:i32,
}

async fn create_task(
    State(pg_pool): State<PgPool>,
    Json(task): Json<CreateTaskReq>
)-> Result<(StatusCode, String), (StatusCode, String)>{
    let row = sqlx::query_as!(
        CreateTaskRow,
        "INSERT INTO tasks (name, priority) VALUES ($1, $2) RETURNING task_id",
        task.name,
        task.priority,)
        .fetch_one(&pg_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "data": row}).to_string(),
    ))
}

#[derive(Deserialize)]

struct UpdateTaskReq{
    name: Option<String>,
    priority: Option<i32>,
}

async fn update_tasks(
    State(pg_pool): State<PgPool>,
    Path(task_id): Path<i32>,
    Json(task): Json<UpdateTaskReq>
)-> Result<(StatusCode, String), (StatusCode, String)> {
    sqlx::query!(
        "
UPDATE tasks SET
name = $2,
priority = $3
WHERE task_id = $1
        ",
        task_id,
        task.name,
        task.priority
    ).execute(&pg_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "message": "Data modified"}).to_string()
    ))
}

async fn delete_task(
    State(pg_pool): State<PgPool>,
    Path(task_id): Path<i32>
) -> Result<(StatusCode, String), (StatusCode, String)> {
    sqlx::query!("DELETE FROM tasks WHERE task_id = $1", task_id)
        .execute(&pg_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "message": "Data deleted"}).to_string()
    ))
}