use std::{ops::Deref, sync::Arc};

use sqlx::{Error, Pool, SqlitePool,Executor,Row};
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
            ON CONFLICT(chat_id,date) DO UPDATE SET total_seconds=excluded.total_seconds
                                ",
                    )
                        .bind(chat_id)
                        .bind(total_seconds),
                )
                .await?;
            Ok(())
    }

    pub async fn get_average_total_per_day_by_chat(self: &Self) -> Result<Vec<(i64, Option<i64>)>, Error> {
        let rows = sqlx::query("SELECT chat_id, AVG(total_seconds) FROM total GROUP BY chat_id")
            .fetch_all(&self.pool)
            .await?;

        let mut result = Vec::new();
        for row in rows {
            let chat_id: i64 = row.try_get(0)?;
            let avg_total_seconds: Option<f64> = row.try_get(1)?;
            result.push((chat_id, avg_total_seconds.map(|x| x as i64)));
        }

        Ok(result)
    }

    pub async fn get_total_seconds_grouped_by_chat(self: &Self) -> Result<Vec<(i64, Option<i64>)>, Error> {
        let rows = sqlx::query("SELECT chat_id, SUM(total_seconds) FROM total GROUP BY chat_id")
            .fetch_all(&self.pool)
            .await?;

        let mut result = Vec::new();
        for row in rows {
            let chat_id: i64 = row.try_get(0)?;
            let avg_total_seconds: Option<i64> = row.try_get(1)?;
            result.push((chat_id, avg_total_seconds.map(|x| x as i64)));
        }

        Ok(result)
    }

    pub async fn get_total_timestamp_day(&self, timestamp: i64, ChatId(chat_id): ChatId) -> Result<Option<i64>, Error> {
        #[derive(sqlx::FromRow)]
        struct TotalSecondsDbRow {
            total_seconds: i64,
        }
        let bytes = sqlx::query_as::<_, TotalSecondsDbRow>(
            "SELECT total_seconds FROM total WHERE chat_id = ? and date = date(?, 'unixepoch')"
        )
            .bind(chat_id)
            .bind(timestamp)
            .fetch_optional(&self.pool)
            .await?
            .map(|r| r.total_seconds);

        Ok(bytes)
    }

}

#[cfg(test)]
mod test_management {
    use chrono::Utc;

    use super::*;

    #[tokio::test]
    async fn test_creating() {
        let total = Total::create_table(":memory:").await.unwrap();
        total.clone().set_total_today(ChatId(1), 100).await.unwrap();

        assert_eq!(total.clone().get_total_timestamp_day(Utc::now().timestamp(),ChatId(1)).await.unwrap().unwrap(), 100);
    }

    #[tokio::test]
    async fn test_adding_yesterdays_dates() {
        let total = Total::create_table(":memory:").await.unwrap();
        sqlx::query("INSERT INTO total VALUES (1, date('now', '-5 day'), 100)")
            .execute(&total.pool)
            .await
            .unwrap();
        assert_eq!(total.clone().get_total_timestamp_day(Utc::now().timestamp(),ChatId(1)).await.unwrap(), None);
        total.clone().set_total_today(ChatId(1), 200).await.unwrap();
        assert_eq!(total.clone().get_total_timestamp_day(Utc::now().timestamp(),ChatId(1)).await.unwrap().unwrap(), 200);
    }

    #[tokio::test]
    async fn test_total_yesterdays_dates() {
        let total = Total::create_table(":memory:").await.unwrap();
        sqlx::query("INSERT INTO total VALUES (1, date('now', '-5 day'), 100)")
            .execute(&total.pool)
            .await
            .unwrap();
        let five_days_ago = Utc::now() - chrono::Duration::days(5);
        assert_eq!(total.clone().get_total_timestamp_day(five_days_ago.timestamp(), ChatId(1)).await.unwrap(), Some(100));
    }

    #[tokio::test]
    async fn test_average() {
        let total = Total::create_table(":memory:").await.unwrap();
        total.clone().set_total_today(ChatId(1), 100).await.unwrap();
        total.clone().set_total_today(ChatId(1), 200).await.unwrap();
        total.clone().set_total_today(ChatId(2), 300).await.unwrap();

        sqlx::query("INSERT INTO total VALUES (2, date('now', '-1 day'), 100)")
            .execute(&total.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO total VALUES (2, date('now', '-2 day'), 200)")
            .execute(&total.pool)
            .await
            .unwrap();

        let averages = total.get_average_total_per_day_by_chat().await.unwrap();

        assert_eq!(averages.len(), 2);
        assert_eq!(averages[0].0, 1);
        assert_eq!(averages[0].1, Some(200));
        assert_eq!(averages[1].0, 2);
        assert_eq!(averages[1].1, Some(250));
    }

    #[tokio::test]
    async fn test_total() {
        let total = Total::create_table(":memory:").await.unwrap();
        total.clone().set_total_today(ChatId(1), 100).await.unwrap();
        total.clone().set_total_today(ChatId(1), 200).await.unwrap();
        total.clone().set_total_today(ChatId(2), 300).await.unwrap();

        sqlx::query("INSERT INTO total VALUES (2, date('now', '-1 day'), 100)")
            .execute(&total.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO total VALUES (2, date('now', '-2 day'), 200)")
            .execute(&total.pool)
            .await
            .unwrap();

        let totals = total.get_total_seconds_grouped_by_chat().await.unwrap();

        assert_eq!(totals.len(), 2);
        assert_eq!(totals[0].0, 1);
        assert_eq!(totals[0].1, Some(200));
        assert_eq!(totals[1].0, 2);
        assert_eq!(totals[1].1, Some(600));
    }

}
