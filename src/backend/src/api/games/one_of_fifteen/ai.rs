use crate::api::games::one_of_fifteen::types::Question;
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
