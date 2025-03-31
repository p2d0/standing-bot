mod periodic_updates;
mod total_management;
mod time;
mod sticker_handling;
mod message_handling;
mod openrouter;

use std::{ops::Deref, sync::Arc};

use periodic_updates::update_periodically;

use serde::Serialize;
use sqlx::{Error, Pool, SqlitePool};
use teloxide::{
    dispatching::{dialogue::{self, serializer::Json, ErasedStorage, SqliteStorage, Storage}, MessageFilterExt, UpdateHandler}, prelude::*, types::{ButtonRequest, ChatMemberStatus, KeyboardButton, KeyboardButtonRequestChat, KeyboardMarkup, MessageChatShared, MessageKind, RequestId}, update_listeners::webhooks, utils::command::BotCommands
};
use total_management::Total;

type MyDialogue = Dialogue<State, ErasedStorage<State>>;
type MyStorage = std::sync::Arc<ErasedStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;


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
    Rankings
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    let path = "dialogues.sqlite";

    let storage: MyStorage = SqliteStorage::open(path, Json).await.unwrap().erase();

    let total_manager = Total::create_table(path).await.unwrap();
    let tx = update_periodically(bot.clone()).await;

    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![storage,tx,total_manager.clone()])
        .enable_ctrlc_handler()
        .build();


    #[cfg(not(debug_assertions))]
    {
        let port = 9999;
        let addr = ([0, 0, 0, 0], port).into();
        let link = "https://bots.upgradegamma.ru/standing_bot";

        let listener = webhooks::axum(bot.clone(),webhooks::Options::new(addr,link.parse().unwrap())).await.expect("Failed to start webhook listener");
        let error_handler =
            LoggingErrorHandler::with_custom_text("An error from the update listener");
        dispatcher.dispatch_with_listener(listener, error_handler).await;
    }

    #[cfg(debug_assertions)]
    {
        dispatcher.dispatch().await;
    }
}


fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Rankings].endpoint(rankings))
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .inspect(|u: Update| {
            log::info!("{u:#?}");
        })
        .branch(command_handler.clone())
        .branch(dptree::filter_map(|message: Message| {
            match message.kind {
                MessageKind::ChatShared(x) => Some(x),
                _ => None,
            }
        }).endpoint(chat_shared))
        .branch(case![State::StandingChoice { chat_id }]
                .endpoint(message_handling::standing_choice))
        .branch(case![State::ReceiveStandingCommand { chat_id , timestamp }].endpoint(message_handling::receive_sit_command))
        .branch(dptree::endpoint(invalid_state));

    let channel_handler = Update::filter_channel_post()
        .branch(command_handler.clone())
        .branch(
            Message::filter_sticker()
                .inspect(|u: Message| {
                    log::info!("{}",u.sticker().unwrap().file.unique_id);
                })
                .branch(case![State::ReceiveStandingCommand { chat_id , timestamp }].endpoint(sticker_handling::standing_status_handler))
                .endpoint(sticker_handling::start_standing_handler))
        .branch(Message::filter_text().branch(case![State::ReceiveStandingCommand { chat_id , timestamp }].endpoint(message_handling::stop_standing)));

    dialogue::enter::<Update, ErasedStorage<State>, State, _>()
        .branch(channel_handler)
        .branch(message_handler)
}


async fn rankings(bot: Bot, msg: Message, total_manager: Arc<Total>) -> HandlerResult {
    let averages = total_manager.get_average_total_per_day_by_chat().await?;
    let mut messages = Vec::new();

    let mut winning_chat: Option<(i64, i64)> = None;

    for (chat_id, average) in averages.iter() {
        let chat = bot.get_chat(ChatId(*chat_id)).await?;
        let chat_name = chat.title().unwrap_or_else(|| chat.username().unwrap_or("Нет имени"));
        let average_seconds = average.unwrap_or(0) as i64;
        messages.push(format!("Чат: {}, Среднее стояние: \n<b>{}</b>", chat_name, time::total_seconds_to_hms(average_seconds)));

        if let Some((_, current_winning_average)) = winning_chat {
            if average_seconds > current_winning_average {
                winning_chat = Some((*chat_id, average_seconds));
            }
        } else {
            winning_chat = Some((*chat_id, average_seconds));
        }
    }

    if let Some((chat_id, average)) = winning_chat {
        let chat = bot.get_chat(ChatId(chat_id)).await?;
        let chat_name = chat.title().unwrap_or_else(|| chat.username().unwrap_or("Нет имени"));
        messages.push(format!("
🏆 <b>Победитель:</b> {} со средним стоянием: <b>{}</b> 🏆", chat_name, time::total_seconds_to_hms(average)));
    }

    bot.send_message(msg.chat.id, messages.join("\n"))
       .parse_mode(teloxide::types::ParseMode::Html)
       .await?;

    Ok(())
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


async fn chat_shared(bot: Bot, msg: Message, dialogue: MyDialogue, chat: MessageChatShared) -> HandlerResult {
    bot.send_message(msg.chat.id, format!("Текущий чат: {}",chat.chat_shared.chat_id))
       .reply_markup(
           KeyboardMarkup::new([[
               KeyboardButton::new("СТОИМ БРАТЬЯ"),
           ]])).await?;
    dialogue.update(State::StandingChoice { chat_id: chat.chat_shared.chat_id }).await?;
    Ok(())
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
