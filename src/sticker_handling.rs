use std::{error::Error, sync::Arc};

use chrono::Utc;
use teloxide::{
    dispatching::dialogue::GetChatId, prelude::*
};
use tokio::sync::watch;

use crate::{periodic_updates::UpdateData, time::{get_seconds_difference, get_seconds_difference_from_now, get_time_difference, total_seconds_to_hms}, total_management::Total, HandlerResult, MyDialogue, State};

pub const STICKER_STAND: &str = "AgADUW0AAk1IgUo";
const SIT_STICKERS_SET: [&str; 5] =
    ["AgADlmEAAlOI8Eo", // Sit wrong pack
     "AgADF3IAAgh4iEo", // Sit punisher
     "AgAD5WYAAtxw-Uo", // chill
     "AgADP24AAn23-Eo", // sit
     "AgADYmMAAhK2qUo" // Laying down
    ];

pub async fn standing_status_handler(bot: Bot,
                                     dialogue: MyDialogue,
                                     msg: Message,
                                     (chat_id, timestamp): (ChatId,i64),
                                     tx: watch::Sender<UpdateData>,
                                     total_manager: Arc<Total>
) -> HandlerResult {
    if let Some(sticker) = msg.sticker() {
        if SIT_STICKERS_SET.contains(&sticker.file.unique_id.as_str()) {
            dialogue.exit().await?;
            let _ = tx.send(UpdateData(None, timestamp));
            bot.unpin_chat_message(msg.chat_id().unwrap()).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp))).await?;

            let total = get_total(total_manager.clone(), chat_id, timestamp).await;
            send_and_update_total(&bot, chat_id, total, total_manager).await?;

        } else {
            bot.send_message(chat_id, format!("СТОИМ {}",get_time_difference(timestamp))).await?;
        }
    }
    Ok(())
}

pub async fn get_total(total_manager: Arc<Total>,chat_id: ChatId,timestamp: i64) -> i64 {
    let total = if let Ok(Some(existing_total)) = total_manager.clone().get_total_today(chat_id).await {
        existing_total + get_seconds_difference_from_now(timestamp)
    } else {
         get_seconds_difference_from_now(timestamp)
    };
    return total;
}

pub async fn send_and_update_total(bot: &Bot, chat_id: ChatId, total: i64, total_manager: Arc<Total>) -> Result<(), Box<dyn Error + Send + Sync>> {
    total_manager.set_total_today(chat_id, total).await?;
    bot.send_message(chat_id, format!("Всего постояли сегодня: {}", total_seconds_to_hms(total))).await?;
    Ok(())
}

pub async fn start_standing_handler(bot: Bot, dialogue: MyDialogue, msg: Message, tx: watch::Sender<UpdateData>) -> HandlerResult {
    match msg.sticker().map(ToOwned::to_owned) {
        Some(sticker) => {
            if sticker.file.unique_id == STICKER_STAND {
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
