use std::{any::Any, error::Error, ops::Deref, sync::Arc};

use chrono::{DateTime, Utc};
use teloxide::{
    dispatching::dialogue::GetChatId, prelude::*
};
use tokio::{sync::{watch, Mutex}, time::{sleep,Duration}};

use crate::time::get_time_difference;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UpdateData(pub Option<Message>, pub i64);

pub async fn periodic_update_msg(bot: Bot, rx: Arc<watch::Receiver<UpdateData>>) {
    loop {
        let UpdateData(message, timestamp) = rx.borrow().clone();
        if let Some(message) = message {
            log::info!("Updating message {:#?}", message);
            let edited_message = format!("Стоим {}", get_time_difference(timestamp));
            if let Err(err) = bot.edit_message_text(message.chat_id().unwrap(),message.id,edited_message).await {
                log::warn!("Failed to update message: {:?}", err);
            };
        }

        sleep(Duration::from_secs(5)).await;
    }
}

pub async fn update_periodically(bot: Bot) -> watch::Sender<UpdateData> {
    let (tx, rx) = watch::channel(UpdateData(None, 0));
    let rx = Arc::new(rx); // Shared state
    let rx_clone = Arc::clone(&rx);

    // tokio::spawn(async move {periodic_update_msg(bot, rx_clone)}.await);
    return tx;
}
