use std::{any::Any, error::Error, ops::Deref, sync::Arc};

use chrono::{DateTime, Utc};
use teloxide::{
    prelude::*, types::{InputFile, KeyboardButton, KeyboardMarkup}
};
use tokio::{sync::{watch, Mutex}, time::{sleep,Duration}};

use crate::{periodic_updates::UpdateData, sticker_handling::STICKER_STAND, time::get_time_difference, HandlerResult, MyDialogue, State};

pub async fn standing_choice(bot: Bot, dialogue: MyDialogue, msg: Message, chat_id: ChatId, tx: watch::Sender<UpdateData>) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(full_name) => {
            if full_name == "СТОИМ БРАТЬЯ" {
                let standing_msg = bot.send_message(chat_id, "СТОИМ БРАТЬЯ").await?;
                bot.pin_chat_message(standing_msg.chat.id, standing_msg.id).await?;
                let timestamp = Utc::now().timestamp();
                let _ = tx.send(UpdateData(Some(standing_msg), timestamp));

                bot.send_sticker(chat_id, InputFile::file_id(STICKER_STAND)).await?;
                bot.send_message(msg.chat.id, "СТОИМ БРАТЬЯ")
                   .reply_markup(
                       KeyboardMarkup::new([[
                           KeyboardButton::new("СИДИМ"),
                       ]])).await?;
                dialogue.update(State::ReceiveStandingCommand { chat_id, timestamp: Utc::now().timestamp() }).await?;
            }
        }
        None => {}
    }
    Ok(())
}


pub async fn start_standing(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64)) -> HandlerResult {
    if let Some(full_name) = msg.text().map(ToOwned::to_owned) {
        if full_name == "СИДИМ" {
            dialogue.update(State::StandingChoice { chat_id }).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp))).await?;

            bot.send_message(msg.chat.id, "СИДИМ")
               .reply_markup(KeyboardMarkup::new([[
                   KeyboardButton::new("СТОИМ БРАТЬЯ"),
               ]]))
               .await?;
        }
    }
    Ok(())
}
