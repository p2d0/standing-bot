use std::{any::Any, error::Error, ops::Deref, sync::Arc};

use chrono::{DateTime, Utc};
use teloxide::{
    dispatching::dialogue::GetChatId, prelude::*, types::{InputFile, KeyboardButton, KeyboardMarkup}
};
use tokio::{sync::{watch, Mutex}, time::{sleep,Duration}};

use crate::{openrouter, periodic_updates::UpdateData, sticker_handling::{get_total, send_and_update_total, STICKER_STAND}, time::get_time_difference, total_management::{self, Total}, HandlerResult, MyDialogue, State};

pub async fn standing_choice(bot: Bot, dialogue: MyDialogue, msg: Message, chat_id: ChatId, tx: watch::Sender<UpdateData>) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(full_name) => {
            if full_name == "СТОИМ БРАТЬЯ" {
                let standing_msg = bot.send_message(chat_id, "СТОИМ БРАТЬЯ").await?;
                bot.pin_chat_message(standing_msg.chat.id, standing_msg.id).await?;
                let timestamp = msg.date.timestamp();
                let _ = tx.send(UpdateData(Some(standing_msg), timestamp));

                bot.send_sticker(chat_id, InputFile::file_id(STICKER_STAND)).await?;
                bot.send_message(msg.chat.id, "СТОИМ БРАТЬЯ")
                   .reply_markup(
                       KeyboardMarkup::new([[
                           KeyboardButton::new("СИДИМ"),
                       ]])).await?;
                dialogue.update(State::ReceiveStandingCommand { chat_id, timestamp: timestamp }).await?;
            }
        }
        None => {}
    }
    Ok(())
}


pub async fn receive_sit_command(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64), total_manager: Arc<Total>) -> HandlerResult {
    if let Some(text) = msg.text().map(ToOwned::to_owned) {
        if text == "СИДИМ" {
            dialogue.update(State::StandingChoice { chat_id }).await?;
            let end_timestamp = msg.date.timestamp();
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp,end_timestamp))).await?;

            bot.send_message(msg.chat.id, "СИДИМ")
               .reply_markup(KeyboardMarkup::new([[
                   KeyboardButton::new("СТОИМ БРАТЬЯ"),
               ]]))
               .await?;
            let total = get_total(total_manager.clone(), chat_id, timestamp,end_timestamp).await;
            send_and_update_total(&bot, chat_id, total,total_manager).await?;
        }
    }
    Ok(())
}

pub async fn stop_standing(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64), tx: watch::Sender<UpdateData>, total_manager: Arc<Total>) -> HandlerResult {
    if let Some(text) = msg.text().map(ToOwned::to_owned) {
        if openrouter::is_intent_to_sit(&text).await.unwrap() {
            dialogue.exit().await?;
            // NOTE Duplication
            let end_timestamp = msg.date.timestamp();
            let _ = tx.send(UpdateData(None, timestamp));
            bot.unpin_chat_message(msg.chat_id().unwrap()).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp, end_timestamp))).await?;
            let total = get_total(total_manager.clone(), chat_id, timestamp, end_timestamp).await;
            send_and_update_total(&bot, chat_id, total, total_manager).await?;
        }
    }
    Ok(())
}
