use axum::{routing::post, Json, Router};
use log::info;
use regex::Regex;
use reqwest::StatusCode;
use unicode_segmentation::UnicodeSegmentation;

pub fn get_routes() -> Router {
    Router::new()
        .route("/15/nice", post(nice))
        .route("/15/game", post(game))
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct Password {
    input: String,
}

async fn nice(Json(password): Json<Password>) -> Result<(StatusCode, String), StatusCode> {
    info!("15 nice started");

    let re_vowels = Regex::new(r"[aeiouy]").unwrap();
    let re_doubles = Regex::new(r"(ab|cd|pq|xy)").unwrap();
    if re_vowels.find_iter(password.input.as_str()).count() < 3
        || !has_two_consecutive_chars(password.input.as_str())
        || re_doubles.find(password.input.as_str()) != None
    {
        return Ok((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\"}".to_string(),
        ));
    }

    Ok((StatusCode::OK, "{\"result\":\"nice\"}".to_string()))
}

async fn game(Json(password): Json<Password>) -> Result<String, (StatusCode, String)> {
    info!("15 game started");

    let s = password.input;

    // Rule 1: must be at least 8 characters long
    if s.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"8 chars\"}".to_string(),
        ));
    }

    // Rule 2: must contain uppercase letters, lowercase letters, and digits
    let re_upper = Regex::new(r"[A-Z]").unwrap();
    let re_lower = Regex::new(r"[a-z]").unwrap();
    let re_digits = Regex::new(r"[\d]").unwrap();

    if re_upper.find(s.as_str()) == None
        || re_lower.find(s.as_str()) == None
        || re_digits.find(s.as_str()) == None
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"more types of chars\"}".to_string(),
        ));
    }

    // Rule 3: must contain at least 5 digits
    if re_digits.find_iter(s.as_str()).count() < 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"55555\"}".to_string(),
        ));
    }

    // Rule 4: all integers (sequences of consecutive digits) in the string must add up to 2023
    let re_numbers = Regex::new(r"[\d]+").unwrap();
    let mut sum = 0;
    for number in re_numbers.find_iter(s.as_str()) {
        sum += number.as_str().parse::<u32>().unwrap();
    }

    if sum != 2023 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"math is hard\"}".to_string(),
        ));
    }

    // Rule 5: must contain the letters j, o, and y in that order and in no other order
    let mut joy = (false, false, false);
    let re_joy = Regex::new(r"[joy]").unwrap();
    for c in s.chars() {
        if joy == (false, false, false) && c == 'j' {
            joy.0 = true;
        } else if joy == (true, false, false) && c == 'o' {
            joy.1 = true;
        } else if joy == (true, true, false) && c == 'y' {
            joy.2 = true;
        }
    }
    if joy != (true, true, true) || re_joy.find_iter(s.as_str()).count() != 3 {
        return Err((
            StatusCode::NOT_ACCEPTABLE,
            "{\"result\":\"naughty\", \"reason\":\"not joyful enough\"}".to_string(),
        ));
    }

    // Rule 6: must contain a letter that repeats with exactly one other letter between them (like xyx)
    let mut rule6 = false;
    let graphemes = s
        .graphemes(true)
        .map(|g| g.chars().fold(0, |acc, c| acc + c as u32))
        .collect::<Vec<u32>>();
    for i in 2..graphemes.len() {
        let (g1, g2, g3) = (graphemes[i - 2], graphemes[i - 1], graphemes[i]);
        if g1 == g3
            && (g1 >= b'a' as u32 && g1 <= b'z' as u32 || g1 >= b'A' as u32 && g1 <= b'Z' as u32)
            && (g2 >= b'a' as u32 && g2 <= b'z' as u32 || g2 >= b'A' as u32 && g2 <= b'Z' as u32)
        {
            rule6 = true;
        }
    }
    if !rule6 {
        return Err((
            StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            "{\"result\":\"naughty\", \"reason\":\"illegal: no sandwich\"}".to_string(),
        ));
    }

    // Rule 7: must contain at least one unicode character in the range [U+2980, U+2BFF]
    let left = 0x2980;
    let right = 0x2BFF;
    let mut rule7 = false;
    for g in s.graphemes(true) {
        let gn = g.chars().fold(0, |acc, c| acc + c as u32);
        if gn >= left && gn <= right {
            rule7 = true;
        }
    }
    if !rule7 {
        return Err((
            StatusCode::RANGE_NOT_SATISFIABLE,
            "{\"result\":\"naughty\", \"reason\":\"outranged\"}".to_string(),
        ));
    }

    // Rule 8: must contain at least one emoji
    let left = 0x1F600;
    let right = 0x1F977;
    let mut rule8 = false;
    for g in s.graphemes(true) {
        let gn = g.chars().fold(0, |acc, c| acc + c as u32);
        if gn >= left && gn <= right {
            rule8 = true;
        }
    }
    if !rule8 {
        return Err((
            StatusCode::UPGRADE_REQUIRED,
            "{\"result\":\"naughty\", \"reason\":\"😳\"}".to_string(),
        ));
    }

    // Rule 9: the hexadecimal representation of the sha256 hash of the string must end with an a
    let hash = sha256::digest(s.to_string());
    if hash.chars().last().unwrap() != 'a' {
        return Err((
            StatusCode::IM_A_TEAPOT,
            "{\"result\":\"naughty\", \"reason\":\"not a coffee brewer\"}".to_string(),
        ));
    }

    Ok("{\"result\":\"nice\", \"reason\":\"that's a nice password\"}".to_string())
}

fn has_two_consecutive_chars(s: &str) -> bool {
    let mut chars = s.chars();
    let mut prev = chars.next().unwrap();
    for c in chars {
        if c == prev && c.is_alphabetic() {
            return true;
        }
        prev = c;
    }
    false
}
