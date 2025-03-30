use std::sync::Arc;

use chrono::Utc;
use teloxide::{
    dispatching::dialogue::GetChatId, prelude::*
};
use tokio::sync::watch;

use crate::{periodic_updates::UpdateData, time::{get_seconds_difference, get_time_difference, get_total_string}, total_management::Total, HandlerResult, MyDialogue, State};

pub const STICKER_STAND: &str = "CAACAgIAAyEFAASI1aNsAAIBkGffuc0yHnECp7_WveEQPfDRxiVvAAJRbQACTUiBSuQosFZ33halNgQ";
const SIT_STICKERS_SET: [&str; 5] =
    ["CAACAgIAAyEFAASI1aNsAAIBlmffuuiIMChfPKanMJKmMp_9Dg91AAKWYQACU4jwSvNuavKzs5PgNgQ", // Sit wrong pack
     "CAACAgIAAyEFAASI1aNsAAIByWfipOSvGgT1T-x3TmGJ7RBJYJPTAAIXcgACCHiISr-Bui4IfxjvNgQ", // Sit punisher
     "CAACAgIAAyEFAASI1aNsAAICaGfjBG5ZMKrJhB4uA2yKrFxzgZG8AAIVZwACpFbxSgEYNHLoMv5vNgQ", // chill
     "CAACAgIAAyEFAASI1aNsAAIBy2fipOVNTzcFLFE0pKHHGz6-AAG49gACP24AAn23-EqkgHxR43HUEjYE", // sit
     "CAACAgIAAyEFAASI1aNsAAICamfjBLPQ93gLqPyAtora_tiKVoP4AAJiYwACErapSkjBxa-bLKFuNgQ" // Laying down
    ];

pub async fn standing_status_handler(bot: Bot,
                                     dialogue: MyDialogue,
                                     msg: Message,
                                     (chat_id, timestamp): (ChatId,i64),
                                     tx: watch::Sender<UpdateData>,
                                     total_manager: Arc<Total>
) -> HandlerResult {
    if let Some(sticker) = msg.sticker() {
        if SIT_STICKERS_SET.contains(&sticker.file.id.as_str()) {
            dialogue.exit().await?;
            let _ = tx.send(UpdateData(None, timestamp));
            bot.unpin_chat_message(msg.chat_id().unwrap()).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp))).await?;

            let total = if let Some(existing_total) = total_manager.clone().get_total_today(chat_id).await? {
                existing_total + get_seconds_difference(timestamp)
            } else {
                get_seconds_difference(timestamp)
            };

            total_manager.set_total_today(chat_id, total).await?;
            bot.send_message(chat_id, format!("Всего постояли сегодня: {}", get_total_string(total))).await?;
        } else {
            bot.send_message(chat_id, format!("СТОИМ {}",get_time_difference(timestamp))).await?;
        }
    }
    Ok(())
}

pub async fn start_standing_handler(bot: Bot, dialogue: MyDialogue, msg: Message, tx: watch::Sender<UpdateData>) -> HandlerResult {
    match msg.sticker().map(ToOwned::to_owned) {
        Some(sticker) => {
            if sticker.file.id == STICKER_STAND {
                let chat_id = msg.chat_id().unwrap();
                dialogue.update(State::ReceiveStandingCommand { chat_id, timestamp: Utc::now().timestamp() }).await?;
                let standing_msg = bot.send_message(chat_id, "СТОИМ БРАТЬЯ").await?;
                bot.pin_chat_message(standing_msg.chat.id, standing_msg.id).await?;
                let timestamp = Utc::now().timestamp();
                let _ = tx.send(UpdateData(Some(standing_msg), timestamp));
            }
        }
        None => {}
    }
    Ok(())
}
