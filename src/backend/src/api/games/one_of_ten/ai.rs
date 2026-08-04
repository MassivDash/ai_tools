use crate::api::games::one_of_ten::types::Question;
use regex::Regex;

pub async fn generate_question_ai(
    api_url: &str,
    age: &str,
    past_questions: &[String],
) -> Option<Question> {
    let client = reqwest::Client::new();
    let past_q_text = if past_questions.is_empty() {
        "".to_string()
    } else {
        format!(
            "Do not repeat any of these previous questions: {:?}.",
            past_questions
        )
    };

    let prompt = format!(
        "Generate a single short trivia question suitable for a {} year old. {}. Format the output as JSON with fields 'text' and 'correct_answer'. Example: {{\"text\": \"What color is the sky?\", \"correct_answer\": \"Blue\"}}",
        age, past_q_text
    );

    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": "You are a game show host's assistant. Output valid JSON only." },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "temperature": 0.8
    });

    println!("Sending AI request to: {}", api_url);

    match client.post(api_url).json(&body).send().await {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                    // Robust JSON extraction using Regex
                    // Looks for { ... } potentially surrounded by other text
                    let re = Regex::new(r"\{[\s\S]*\}").unwrap();
                    if let Some(caps) = re.captures(content) {
                        let json_str = caps.get(0).unwrap().as_str();
                        if let Ok(q) = serde_json::from_str::<Question>(json_str) {
                            return Some(q);
                        } else {
                            println!("Failed to parse extracted JSON: {}", json_str);
                        }
                    } else {
                        println!("No JSON object found in response: {}", content);
                    }
                }
            }
        }
        Err(e) => println!("AI Request failed: {}", e),
    }
    None
}

pub async fn validate_answer_ai(
    api_url: &str,
    question: &str,
    correct_answer: &str,
    user_answer: &str,
) -> bool {
    let client = reqwest::Client::new();
    let prompt = format!(
        "Question: {}\nCorrect Answer: {}\nUser's Answer: {}\nIs the user's answer correct? Answer 'yes' or 'no' only. Be lenient with minor typos or paraphrasing.",
        question, correct_answer, user_answer
    );

    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": "You are a game show judge. Determine if answers are correct. Reply with 'yes' or 'no' only." },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "temperature": 0.3
    });

    match client.post(api_url).json(&body).send().await {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                    let answer_lower = content.trim().to_lowercase();
                    return answer_lower.contains("yes");
                }
            }
        }
        Err(e) => println!("AI validation request failed: {}", e),
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{assistant_completion, MockLlm, MockLlmConfig, UNREACHABLE_LLM_URL};

    fn unreachable_completions_url() -> String {
        format!("{}/v1/chat/completions", UNREACHABLE_LLM_URL)
    }

    #[tokio::test]
    async fn test_a_json_reply_becomes_a_question() {
        let llm = MockLlm::start(MockLlmConfig::replying(
            "{\"text\": \"What color is the sky?\", \"correct_answer\": \"Blue\"}",
        ))
        .await;

        let question = generate_question_ai(&llm.chat_url(), "10", &[])
            .await
            .expect("A well-formed reply should produce a question");

        assert_eq!(question.text, "What color is the sky?");
        assert_eq!(question.correct_answer, "Blue");
        assert!(question.options.is_none());

        // The age lands in the prompt, and with no history there is nothing to avoid
        let requests = llm.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["stream"], false);
        assert_eq!(requests[0]["temperature"], 0.8);
        let prompt = requests[0]["messages"][1]["content"]
            .as_str()
            .expect("the prompt should be a string");
        assert!(prompt.contains("suitable for a 10 year old"), "{}", prompt);
        assert!(!prompt.contains("Do not repeat"), "{}", prompt);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_json_wrapped_in_prose_is_still_extracted() {
        let llm = MockLlm::start(MockLlmConfig::replying(
            "Sure! Here you go:\n```json\n{\"text\": \"2 + 2?\", \"correct_answer\": \"4\", \
             \"options\": [\"3\", \"4\"]}\n```\nHope that helps.",
        ))
        .await;

        let question = generate_question_ai(&llm.chat_url(), "7", &[])
            .await
            .expect("The embedded object should be extracted");

        assert_eq!(question.text, "2 + 2?");
        assert_eq!(question.correct_answer, "4");
        assert_eq!(
            question.options,
            Some(vec!["3".to_string(), "4".to_string()])
        );

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_previous_questions_are_listed_in_the_prompt() {
        let llm = MockLlm::start(MockLlmConfig::replying(
            "{\"text\": \"Capital of France?\", \"correct_answer\": \"Paris\"}",
        ))
        .await;

        let past = vec!["What color is the sky?".to_string(), "2 + 2?".to_string()];
        generate_question_ai(&llm.chat_url(), "12", &past)
            .await
            .expect("A well-formed reply should produce a question");

        let prompt = llm.requests()[0]["messages"][1]["content"]
            .as_str()
            .expect("the prompt should be a string")
            .to_string();
        assert!(prompt.contains("Do not repeat"), "{}", prompt);
        assert!(prompt.contains("What color is the sky?"), "{}", prompt);
        assert!(prompt.contains("2 + 2?"), "{}", prompt);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_reply_without_any_json_object_yields_nothing() {
        let llm = MockLlm::start(MockLlmConfig::replying("I would rather not.")).await;

        assert!(generate_question_ai(&llm.chat_url(), "10", &[])
            .await
            .is_none());

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_json_object_with_the_wrong_shape_yields_nothing() {
        let llm = MockLlm::start(MockLlmConfig::replying(
            "{\"question\": \"missing fields\"}",
        ))
        .await;

        assert!(generate_question_ai(&llm.chat_url(), "10", &[])
            .await
            .is_none());

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_reply_that_is_not_json_at_all_yields_nothing() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = "<html>gateway error</html>".to_string();
        let llm = MockLlm::start(config).await;

        assert!(generate_question_ai(&llm.chat_url(), "10", &[])
            .await
            .is_none());
        assert_eq!(llm.call_count(), 1);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unreachable_server_yields_no_question() {
        assert!(
            generate_question_ai(&unreachable_completions_url(), "10", &[])
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_a_yes_verdict_accepts_the_answer() {
        let llm = MockLlm::start(MockLlmConfig::replying("Yes, close enough.")).await;

        assert!(
            validate_answer_ai(&llm.chat_url(), "Capital of Italy?", "Rome", "rome").await,
            "a 'yes' verdict should accept the answer"
        );

        // The judge is given the question and both answers, at a low temperature
        let requests = llm.requests();
        assert_eq!(requests[0]["temperature"], 0.3);
        let prompt = requests[0]["messages"][1]["content"]
            .as_str()
            .expect("the prompt should be a string");
        assert!(prompt.contains("Question: Capital of Italy?"), "{}", prompt);
        assert!(prompt.contains("Correct Answer: Rome"), "{}", prompt);
        assert!(prompt.contains("User's Answer: rome"), "{}", prompt);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_no_verdict_rejects_the_answer() {
        let llm = MockLlm::start(MockLlmConfig::replying("no")).await;

        assert!(!validate_answer_ai(&llm.chat_url(), "Capital of Italy?", "Rome", "Paris").await);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_reply_without_content_rejects_the_answer() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = serde_json::json!({ "choices": [] }).to_string();
        let llm = MockLlm::start(config).await;

        assert!(!validate_answer_ai(&llm.chat_url(), "Capital of Italy?", "Rome", "Rome").await);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unreachable_judge_rejects_the_answer() {
        assert!(
            !validate_answer_ai(
                &unreachable_completions_url(),
                "Capital of Italy?",
                "Rome",
                "Rome"
            )
            .await
        );
    }

    #[tokio::test]
    async fn test_the_first_choice_of_a_multi_choice_reply_is_used() {
        let mut config = MockLlmConfig::replying("ignored");
        let mut body: serde_json::Value = serde_json::from_str(&assistant_completion(
            "{\"text\": \"A?\", \"correct_answer\": \"B\"}",
        ))
        .expect("the canned body should be valid JSON");
        let extra = serde_json::json!({
            "index": 1,
            "message": { "role": "assistant", "content": "{\"text\": \"C?\", \"correct_answer\": \"D\"}" },
            "finish_reason": "stop"
        });
        body["choices"]
            .as_array_mut()
            .expect("choices should be an array")
            .push(extra);
        config.chat_body = body.to_string();
        let llm = MockLlm::start(config).await;

        let question = generate_question_ai(&llm.chat_url(), "10", &[])
            .await
            .expect("The first choice should be used");
        assert_eq!(question.text, "A?");

        llm.stop().await;
    }
}
