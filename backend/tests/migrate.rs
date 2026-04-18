use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn all_migrations_apply_cleanly(pool: PgPool) {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .expect("users table must exist after migrations");
    assert_eq!(row.0, 0, "fresh database should have zero users");

    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens")
        .fetch_one(&pool)
        .await
        .expect("refresh_tokens table must exist");
    assert_eq!(row.0, 0);

    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
        .fetch_one(&pool)
        .await
        .expect("messages table must exist");
    assert_eq!(row.0, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn username_uniqueness_is_case_insensitive(pool: PgPool) {
    sqlx::query(
        "INSERT INTO users (username, email, password_hash)
         VALUES ('Alice', 'alice@example.com', 'hash')"
    )
    .execute(&pool)
    .await
    .expect("insert first user");

    let result = sqlx::query(
        "INSERT INTO users (username, email, password_hash)
         VALUES ('alice', 'alice2@example.com', 'hash')"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "citext UNIQUE must reject case-insensitive duplicate username");
}

#[sqlx::test(migrations = "./migrations")]
async fn account_status_check_constraint_rejects_invalid_values(pool: PgPool) {
    let result = sqlx::query(
        "INSERT INTO users (username, email, password_hash, account_status)
         VALUES ('bob', 'bob@example.com', 'hash', 'hacked')"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "CHECK constraint must reject invalid account_status");
}

#[sqlx::test(migrations = "./migrations")]
async fn server_member_role_check_constraint_rejects_invalid_values(pool: PgPool) {
    let user_id: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, email, password_hash) VALUES ('carol', 'carol@example.com', 'hash') RETURNING id"
    )
    .fetch_one(&pool).await.unwrap();

    let server_id: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO servers (name, owner_id) VALUES ('Test Server', $1) RETURNING id"
    )
    .bind(user_id.0)
    .fetch_one(&pool).await.unwrap();

    let result = sqlx::query(
        "INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'superuser')"
    )
    .bind(server_id.0)
    .bind(user_id.0)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "CHECK constraint must reject invalid role");
}
