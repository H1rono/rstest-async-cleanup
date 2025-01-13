pub mod repository;

#[derive(Debug, Clone)]
pub struct Repository {
    pool: sqlx::MySqlPool,
}
