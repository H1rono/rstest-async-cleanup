use serde::{Deserialize, Serialize};

use crate::Repository;

// MARK: Builder

pub struct Builder<Hostname = (), Username = (), Password = (), Port = (), Database = ()> {
    hostname: Hostname,
    username: Username,
    password: Password,
    port: Port,
    database: Database,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            hostname: (),
            username: (),
            password: (),
            port: (),
            database: (),
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Hostname, Username, Password, Port, Database>
    Builder<Hostname, Username, Password, Port, Database>
{
    pub fn hostname(self, hostname: &str) -> Builder<String, Username, Password, Port, Database> {
        Builder {
            hostname: hostname.to_string(),
            username: self.username,
            password: self.password,
            port: self.port,
            database: self.database,
        }
    }

    pub fn username(self, username: &str) -> Builder<Hostname, String, Password, Port, Database> {
        Builder {
            hostname: self.hostname,
            username: username.to_string(),
            password: self.password,
            port: self.port,
            database: self.database,
        }
    }

    pub fn password(self, password: &str) -> Builder<Hostname, Username, String, Port, Database> {
        Builder {
            hostname: self.hostname,
            username: self.username,
            password: password.to_string(),
            port: self.port,
            database: self.database,
        }
    }

    pub fn port(self, port: u16) -> Builder<Hostname, Username, Password, u16, Database> {
        Builder {
            hostname: self.hostname,
            username: self.username,
            password: self.password,
            port,
            database: self.database,
        }
    }

    pub fn database(self, database: &str) -> Builder<Hostname, Username, Password, Port, String> {
        Builder {
            hostname: self.hostname,
            username: self.username,
            password: self.password,
            port: self.port,
            database: database.to_string(),
        }
    }
}

impl Builder<String, String, String, u16, String> {
    pub async fn build(self) -> sqlx::Result<Repository> {
        let options = sqlx::mysql::MySqlConnectOptions::new()
            .host(&self.hostname)
            .username(&self.username)
            .password(&self.password)
            .port(self.port)
            .database(&self.database);
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_with(options)
            .await?;
        Ok(Repository { pool })
    }
}

impl Repository {
    pub fn builder() -> Builder {
        Builder::new()
    }
}

// MARK: Migration

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

impl Builder<String, String, String, u16, String> {
    pub async fn build_with_migrate(self) -> sqlx::Result<Repository> {
        let repository = self.build().await?;
        repository.migrate().await?;
        Ok(repository)
    }
}

impl Repository {
    #[tracing::instrument(skip(self))]
    pub async fn migrate(&self) -> sqlx::Result<()> {
        MIGRATOR.run(&self.pool).await?;
        tracing::info!("Database migration completed");
        Ok(())
    }
}

// MARK: Entry definition

pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entry {
    pub id: uuid::Uuid,
    pub key: String,
    pub value: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub deleted_at: Option<DateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, sqlx::FromRow)]
struct DbEntry {
    id: uuid::Uuid,
    key: String,
    value: String,
    created_at: DateTime,
    updated_at: DateTime,
    deleted_at: Option<DateTime>,
}

impl From<Entry> for DbEntry {
    fn from(entry: Entry) -> Self {
        Self {
            id: entry.id,
            key: entry.key,
            value: entry.value,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            deleted_at: entry.deleted_at,
        }
    }
}

impl From<DbEntry> for Entry {
    fn from(entry: DbEntry) -> Self {
        Self {
            id: entry.id,
            key: entry.key,
            value: entry.value,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            deleted_at: entry.deleted_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CreateEntry {
    pub key: String,
    pub value: String,
}

// MARK: Entry operations

impl Repository {
    #[tracing::instrument(skip_all)]
    pub async fn create_entry(&self, entry: CreateEntry) -> sqlx::Result<Entry> {
        let id = uuid::Uuid::now_v7();
        let CreateEntry { key, value } = entry;
        tracing::debug!(%id, "Creating entry");
        let _res = sqlx::query(
            r#"
            INSERT INTO `entries` (`id`, `key`, `value`)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        tracing::debug!(%id, "Entry created");
        let entry: DbEntry = sqlx::query_as(
            r#"
            SELECT * FROM `entries`
            WHERE `id` = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(entry.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_entry(&self, id: uuid::Uuid) -> sqlx::Result<Option<Entry>> {
        let entry: Option<DbEntry> = sqlx::query_as(
            r#"
            SELECT * FROM `entries`
            WHERE `id` = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        tracing::debug!(exists = entry.is_some(), "Fetched entry");
        Ok(entry.map(Entry::from))
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_entries(&self) -> sqlx::Result<Vec<Entry>> {
        let entries: Vec<DbEntry> = sqlx::query_as(
            r#"
            SELECT * FROM `entries`
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        tracing::debug!(count = entries.len(), "Fetched entries");
        Ok(entries.into_iter().map(Entry::from).collect())
    }

    #[tracing::instrument(skip(self, entry))]
    pub async fn update_entry(
        &self,
        id: uuid::Uuid,
        entry: CreateEntry,
    ) -> sqlx::Result<Option<Entry>> {
        let CreateEntry { key, value } = entry;
        tracing::debug!("Updating entry");
        let _res = sqlx::query(
            r#"
            UPDATE `entries`
            SET `key` = ?, `value` = ?, `updated_at` = NOW()
            WHERE `id` = ?
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(id)
        .execute(&self.pool)
        .await?;
        tracing::debug!("Entry updated");
        let entry: Option<DbEntry> = sqlx::query_as(
            r#"
            SELECT * FROM `entries`
            WHERE `id` = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(entry.map(Entry::from))
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_entry(&self, id: uuid::Uuid) -> sqlx::Result<Option<Entry>> {
        tracing::debug!("Deleting entry");
        // TODO: check rows affected to return None if no rows were updated
        let res = sqlx::query(
            r#"
            UPDATE `entries`
            SET `deleted_at` = NOW()
            WHERE `id` = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        tracing::debug!(result = ?res, "Entry deleted");
        let entry: DbEntry = sqlx::query_as(
            r#"
            SELECT * FROM `entries`
            WHERE `id` = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(entry.into()))
    }
}

// MARK: tests

#[cfg(test)]
fn setup_tracing_subscriber() {
    use std::sync::OnceLock;
    use tracing_subscriber::{fmt, EnvFilter};

    static SUBSCRIBER: OnceLock<()> = OnceLock::new();

    *SUBSCRIBER.get_or_init(|| {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_default();
        fmt().with_env_filter(env_filter).init();
    })
}

#[cfg(test)]
mod delete_after_test {
    use tokio::sync::oneshot;

    struct SendOnDrop {
        tx: Option<oneshot::Sender<()>>,
    }

    impl Drop for SendOnDrop {
        fn drop(&mut self) {
            let Some(tx) = self.tx.take() else {
                return;
            };
            match tx.send(()) {
                Ok(()) => tracing::trace!("Signal sent"),
                Err(()) => tracing::error!("Failed to send signal"),
            };
        }
    }

    pub struct DeleteAfterTest {
        _guard: SendOnDrop,
    }

    #[tracing::instrument(
        name = "delete_after_test",
        skip(repository, entry),
        fields(id = %entry.id),
    )]
    pub fn new(repository: &super::Repository, entry: &super::Entry) -> DeleteAfterTest {
        let (tx, rx) = oneshot::channel();
        let repository = repository.clone();
        let id = entry.id;
        let _handle = tokio::spawn(async move {
            let received = rx.await;
            tracing::trace!(?received, "Received signal");
            let res = repository.delete_entry(id).await;
            if let Err(err) = res {
                tracing::error!(
                    error = &err as &dyn std::error::Error,
                    "Failed to delete entry"
                );
            } else {
                tracing::info!("Entry deleted");
            }
        });
        let _guard = SendOnDrop { tx: Some(tx) };
        DeleteAfterTest { _guard }
    }
}

#[cfg(test)]
mod tests {
    use super::{delete_after_test, setup_tracing_subscriber};
    use crate::Repository;

    async fn load_repository() -> Repository {
        use tokio::sync::OnceCell;

        static REPOSITORY: OnceCell<Repository> = OnceCell::const_new();

        REPOSITORY
            .get_or_init(|| async {
                Repository::builder()
                    .hostname("localhost")
                    .username("db")
                    .password("password")
                    .port(3306)
                    .database("asynccleanup")
                    .build_with_migrate()
                    .await
                    .unwrap()
            })
            .await
            .clone()
    }

    #[tokio::test]
    async fn test_create_entry() {
        let () = setup_tracing_subscriber();
        let repository = load_repository().await;
        let entry = repository
            .create_entry(super::CreateEntry {
                key: "test".to_string(),
                value: "test".to_string(),
            })
            .await
            .unwrap();
        let _guard = delete_after_test::new(&repository, &entry);
        assert_eq!(entry.key, "test");
        assert_eq!(entry.value, "test");
    }
}
