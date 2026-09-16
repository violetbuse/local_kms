use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let db_path = local_kms::db::default_db_path();
    let pool = local_kms::db::connect_file(&db_path)
        .await
        .expect("failed to connect to database");
    local_kms::db::migrate(&pool).await.expect("failed to run migrations");

    let port: u16 = std::env::var("LOCAL_KMS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let app = local_kms::app(pool);
    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind port");

    println!("local_kms listening on 0.0.0.0:{port}, db at {db_path}");
    axum::serve(listener, app).await.expect("server error");
}
