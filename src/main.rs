use std::error::Error;

use chrono::{DateTime, Utc};
use teloxide::{
    dispatching::{dialogue::{self, serializer::Json, ErasedStorage, GetChatId, SqliteStorage, Storage}, MessageFilterExt, UpdateHandler},
    prelude::*,
    types::{ButtonRequest, InputFile, KeyboardButton, KeyboardButtonRequestChat, KeyboardMarkup, MessageChatShared, MessageKind, RequestId},
    utils::command::BotCommands,
};

type MyDialogue = Dialogue<State, ErasedStorage<State>>;
type MyStorage = std::sync::Arc<ErasedStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

const STICKER_STAND: &str = "CAACAgIAAyEFAASI1aNsAAIBkGffuc0yHnECp7_WveEQPfDRxiVvAAJRbQACTUiBSuQosFZ33halNgQ";
const STICKER_SIT: &str = "CAACAgIAAyEFAASI1aNsAAIBlmffuuiIMChfPKanMJKmMp_9Dg91AAKWYQACU4jwSvNuavKzs5PgNgQ";

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum State {
    #[default]
    Start,
    ReceiveStandingCommand {
        chat_id: ChatId,
        timestamp: i64
    },
    StopStanding {
        chat_id: ChatId
    },
    ReceiveFullName,
    StandingChoice {
        chat_id: ChatId
    },
    ReceiveProductChoice {
        full_name: String,
    },
}

/// These commands are supported:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text.
    Help,
    /// Start the purchase procedure.
    Start,
    /// Cancel the purchase procedure.
    Cancel,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting purchase bot...");

    let bot = Bot::from_env();

    let storage: MyStorage = SqliteStorage::open("dialogues.sqlite", Json).await.unwrap().erase();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}


fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
    // .inspect(|u: Update| {
    //     eprintln!("{u:#?}"); // Print the update to the console with inspect
    // })
        .branch(command_handler)
        .branch(dptree::filter_map(|message: Message| {
            match message.kind {
                MessageKind::ChatShared(x) => Some(x),
                _ => None,
            }
        }).endpoint(chat_shared))
        .branch(case![State::StandingChoice { chat_id }]
                .endpoint(standing_choice))
        .branch(case![State::ReceiveStandingCommand { chat_id , timestamp }].endpoint(start_standing))
        .branch(dptree::endpoint(invalid_state));

    let sticker_handler = Update::filter_channel_post()
    // .inspect(|u: Update| {
    //     eprintln!("{u:#?}"); // Print the update to the console with inspect
    // })
        .branch(Message::filter_sticker()
                .branch(case![State::ReceiveStandingCommand { chat_id , timestamp }].endpoint(receive_start_standing_sticker))
                .endpoint(start_standing_sticker));

    dialogue::enter::<Update, ErasedStorage<State>, State, _>()
        .branch(sticker_handler)
        .branch(message_handler)
}

async fn start(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Скинь чат бро")
       .reply_markup(
           KeyboardMarkup::new([[
               KeyboardButton::new("Отправить чат").request(ButtonRequest::RequestChat(KeyboardButtonRequestChat::new(RequestId(1), true))),
           ]])).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn start_standing_sticker(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.sticker().map(ToOwned::to_owned) {
        Some(sticker) => {
            if sticker.file.id == STICKER_STAND {
                let chat_id = msg.chat_id().unwrap();
                dialogue.update(State::ReceiveStandingCommand { chat_id, timestamp: Utc::now().timestamp() }).await?;
                bot.send_message(chat_id, "СТОИМ БРАТЬЯ").await?;
            }
        }
        None => {}
    }
    Ok(())
}

async fn chat_shared(bot: Bot, msg: Message, dialogue: MyDialogue, chat: MessageChatShared) -> HandlerResult {
    bot.send_message(msg.chat.id, format!("Текущий чат: {}",chat.chat_shared.chat_id))
       .reply_markup(
           KeyboardMarkup::new([[
               KeyboardButton::new("СТОИМ БРАТЬЯ"),
           ]])).await?;
    dialogue.update(State::StandingChoice { chat_id: chat.chat_shared.chat_id }).await?;
    Ok(())
}

async fn standing_choice(bot: Bot, dialogue: MyDialogue, msg: Message, chat_id: ChatId) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(full_name) => {
            if full_name == "СТОИМ БРАТЬЯ" {
                bot.send_message(chat_id, "СТОИМ БРАТЬЯ").await?;
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

async fn receive_start_standing_sticker(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64)) -> HandlerResult {
    if let Some(sticker) = msg.sticker() {
        if sticker.file.id == STICKER_SIT {
            dialogue.exit().await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp).unwrap_or("".into()))).await?;
        } else {
            bot.send_message(chat_id, format!("СТОИМ {}",get_time_difference(timestamp).unwrap_or("".into()))).await?;
        }
    }
    Ok(())
}


async fn start_standing(bot: Bot, dialogue: MyDialogue, msg: Message, (chat_id, timestamp): (ChatId,i64)) -> HandlerResult {
    if let Some(full_name) = msg.text().map(ToOwned::to_owned) {
        if full_name == "СИДИМ" {
            dialogue.update(State::StandingChoice { chat_id }).await?;
            bot.send_message(chat_id, format!("ПОСТОЯЛИ {}",get_time_difference(timestamp).unwrap_or("".into()))).await?;

            bot.send_message(msg.chat.id, "СИДИМ")
               .reply_markup(KeyboardMarkup::new([[
                   KeyboardButton::new("СТОИМ БРАТЬЯ"),
               ]]))
               .await?;
        }
    }
    Ok(())
}

fn get_time_difference(timestamp: i64) -> Option<String> {
    let now = Utc::now();
    if let Some(past) = DateTime::from_timestamp(timestamp,0) {
        let difference = now.signed_duration_since(past);
        let minutes = difference.num_minutes();
        let seconds = difference.num_seconds();
        return Some(format!("{minutes} минут {seconds} секунд"));
    }
    None
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Cancelling the dialogue.").await?;
    dialogue.exit().await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Unable to handle the message. Type /help to see the usage.")
       .await?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use dptree::deps;
    use teloxide::dispatching::{dialogue::{InMemStorage}};
    use teloxide_tests::{MockBot, MockMessageText};

    #[tokio::test]
    async fn test_start_tree() {
        let bot = MockBot::new(MockMessageText::new().text("/start"), schema());
        let storage: MyStorage = InMemStorage::new().erase();
        bot.dependencies(deps![storage]);
        bot.dispatch_and_check_last_text_and_state("Скинь чат бро",State::Start).await;
        // bot.update(MockMessageText::new().text("kekes"));
        // bot.dispatch_and_check_last_text_and_state("Your tasks: kekes",State::AskForText).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_stand_brothers() {
        // let bot = MockBot::new(MockMessageText::new().text("/start"), schema());
        // let storage: MyStorage = InMemStorage::new().erase();
        // bot.dependencies(deps![storage]);
        // bot.dispatch_and_check_last_text_and_state("Скинь чат бро",State::Start).await;
        // bot.update(MockMessageText::new().text("kekes"));
        // bot.dispatch_and_check_last_text_and_state("Your tasks: kekes",State::AskForText).await;
    }
}
