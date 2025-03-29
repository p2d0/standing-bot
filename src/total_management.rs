use std::{ops::Deref, sync::Arc};

use sqlx::{Error, Pool, SqlitePool,Executor};
use teloxide::types::ChatId;


#[derive(Clone)]
pub struct Total {
    pool: Pool<sqlx::Sqlite>
}

impl Total {
    pub async fn create_table(path: &str) -> Result<Arc<Self>, Error> {
        let pool = SqlitePool::connect(format!("sqlite:{path}?mode=rwc").as_str()).await?;
        sqlx::query(
            "
CREATE TABLE IF NOT EXISTS total (
    chat_id BIGINT PRIMARY KEY,
    date TEXT,
    total_seconds INT
);
        ").execute(&pool)
            .await?;
        return Ok(Arc::new(Self {pool}));
    }

    pub async fn set_total_today(self: Arc<Self>,ChatId(chat_id):ChatId,total_seconds: i64) -> Result<(),Error>
    {
            self.pool
                .acquire()
                .await?
                .execute(
                    sqlx::query(
                        "
            INSERT INTO total VALUES (?, date('now'), ?)
            ON CONFLICT(chat_id) DO UPDATE SET total_seconds=excluded.total_seconds
                                ",
                    )
                        .bind(chat_id)
                        .bind(total_seconds),
                )
                .await?;
            Ok(())
    }

    pub async fn get_total_today(self: Arc<Self>, ChatId(chat_id):ChatId) -> Result<Option<i64>,Error>
    {
        #[derive(sqlx::FromRow)]
        struct TotalSecondsDbRow {
            total_seconds: i64,
        }
        let bytes = sqlx::query_as::<_, TotalSecondsDbRow>(
            "SELECT total_seconds FROM total WHERE chat_id = ? and date = date('now')"
        )
            .bind(chat_id)
            .fetch_optional(&self.pool)
            .await?
            .map(|r| r.total_seconds);

        Ok(bytes)
    }

}
