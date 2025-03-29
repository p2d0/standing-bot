use std::{any::Any, error::Error, ops::Deref, sync::Arc};

use chrono::{DateTime, Utc};
use teloxide::{
    dispatching::dialogue::GetChatId, prelude::*, types::{InputFile, KeyboardButton, KeyboardMarkup}
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


pub async fn receive_sit_command(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64)) -> HandlerResult {
    if let Some(text) = msg.text().map(ToOwned::to_owned) {
        if text == "СИДИМ" {
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

pub async fn stop_standing(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64), tx: watch::Sender<UpdateData>) -> HandlerResult {
    if let Some(text) = msg.text().map(ToOwned::to_owned) {
        if text.to_lowercase() == "чил" {
            dialogue.exit().await?;
            // NOTE Duplication
            let _ = tx.send(UpdateData(None, timestamp));
            bot.unpin_chat_message(msg.chat_id().unwrap()).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp))).await?;
        }
    }
    Ok(())
}
