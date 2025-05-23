use reqwest;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use dotenv::dotenv;

pub async fn is_intent_to_sit(message: &str) -> Result<bool, Box<dyn Error>> {
    let api_key = env::var("OPENROUTER_API_KEY")
        .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?;

    let client = reqwest::Client::new();

    let base_prompt = "Analyze the message if the intent of the message to sit/relax in present moment not in the future or the past and not stand/standing up and not any other intent only reply with 1 else 0.
Only reply with 1 if you're sure
Examples:
ну ща не надолго, лежать пойду уже - 0
Чилим - 1
Чил - 1
Message:
 ";
    let full_prompt = format!("{}{}", base_prompt, message);

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "google/gemini-2.0-flash-001",
            "messages": [
                {
                    "role": "user",
                    "content": full_prompt
                }
            ]
        }))
        .send()
        .await?;

    let response_text: Value = response.json().await?;
    let result = response_text["choices"][0]["message"]["content"]
        .as_str()
        .unwrap().trim() == "1";

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // Load .env file before each test
    fn setup() {
        dotenv().ok(); // Load .env file, fail test if not present
        // env_logger::init();
    }

    #[tokio::test]
    async fn test_sit_intent() {
        setup();

        let result = is_intent_to_sit("Чилим")
            .await
            .expect("Function should not error");

        assert_eq!(result, true, "Should return true for Чил");

        let result = is_intent_to_sit("ну ща не надолго, лежать пойду уже")
            .await
            .expect("Function should not error");

        assert_eq!(result, false, "Should return true for sit intent");

    }

    #[tokio::test]
    async fn test_stand_intent() {
        setup();

        let result = is_intent_to_sit("Please stand up")
            .await
            .expect("Function should not error");

        assert_eq!(result, false, "Should return false for stand intent");
    }
}
