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
    chat_id BIGINT,
    date TEXT,
    total_seconds INT,
    CONSTRAINT id_date UNIQUE(chat_id, date)
);
        ").execute(&pool)
            .await?;
        return Ok(Arc::new(Self {pool}));
    }

    pub async fn set_total_today(self: &Self,ChatId(chat_id):ChatId,total_seconds: i64) -> Result<(),Error>
    {
            self.pool
                .acquire()
                .await?
                .execute(
                    sqlx::query(
                        "
            INSERT INTO total VALUES (?, date('now'), ?)
            ON CONFLICT(chat_id,date) DO UPDATE SET total_seconds=excluded.total_seconds + total.total_seconds
                                ",
                    )
                        .bind(chat_id)
                        .bind(total_seconds),
                )
                .await?;
            Ok(())
    }

    pub async fn get_total_today(self: &Self, ChatId(chat_id):ChatId) -> Result<Option<i64>,Error>
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

#[cfg(test)]
mod test_management {
    use super::*;

    #[tokio::test]
    async fn test_creating() {
        let total = Total::create_table(":memory:").await.unwrap();
        total.clone().set_total_today(ChatId(1), 100).await.unwrap();
        assert_eq!(total.clone().get_total_today(ChatId(1)).await.unwrap().unwrap(), 100);
    }

    #[tokio::test]
    async fn test_adding_yesterdays_dates() {
        let total = Total::create_table(":memory:").await.unwrap();
        sqlx::query("INSERT INTO total VALUES (1, date('now', '-5 day'), 100)")
            .execute(&total.pool)
            .await
            .unwrap();
        assert_eq!(total.clone().get_total_today(ChatId(1)).await.unwrap(), None);
        total.clone().set_total_today(ChatId(1), 200).await.unwrap();
        assert_eq!(total.clone().get_total_today(ChatId(1)).await.unwrap().unwrap(), 200);
    }
}
