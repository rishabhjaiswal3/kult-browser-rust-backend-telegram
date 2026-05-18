use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct TelegramUser {
    pub id: i64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

/// Verifies a Telegram Mini App `initData` string against the bot token and returns the user.
///
/// Algorithm from Telegram docs:
/// 1. Split initData into key=value pairs
/// 2. Remove the `hash` pair, sort the rest alphabetically
/// 3. Join them as `key=value\n...` (data-check-string)
/// 4. secret_key = HMAC-SHA256(bot_token, key="WebAppData")
/// 5. expected_hash = HMAC-SHA256(data-check-string, key=secret_key)  — encoded as lowercase hex
/// 6. Compare expected_hash with the `hash` from initData
pub fn verify_telegram_init_data(init_data: &str, bot_token: &str) -> Result<TelegramUser, String> {
    let pairs = parse_init_data(init_data);

    let hash = pairs
        .iter()
        .find(|(k, _)| k == "hash")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| "Missing hash field in Telegram initData".to_string())?;

    let mut check_pairs: Vec<&(String, String)> =
        pairs.iter().filter(|(k, _)| k != "hash").collect();
    check_pairs.sort_by(|(a, _), (b, _)| a.cmp(b));

    let data_check_string = check_pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    // Step 1: secret key = HMAC-SHA256(bot_token, key="WebAppData")
    let mut mac = HmacSha256::new_from_slice(b"WebAppData")
        .map_err(|e| format!("HMAC init error: {}", e))?;
    mac.update(bot_token.as_bytes());
    let secret = mac.finalize().into_bytes();

    // Step 2: expected hash = HMAC-SHA256(data_check_string, key=secret)
    let mut mac = HmacSha256::new_from_slice(&secret)
        .map_err(|e| format!("HMAC compute error: {}", e))?;
    mac.update(data_check_string.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex: String = computed.iter().map(|b| format!("{:02x}", b)).collect();

    if computed_hex != hash {
        return Err("Telegram initData HMAC verification failed — invalid or tampered data".to_string());
    }

    // Reject stale initData (>24 hours old)
    if let Some((_, auth_date_str)) = check_pairs.iter().find(|(k, _)| k.as_str() == "auth_date") {
        if let Ok(auth_date) = auth_date_str.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            if now - auth_date > 86_400 {
                return Err("Telegram initData is expired (older than 24 hours)".to_string());
            }
        }
    }

    // Extract the `user` JSON embedded in initData
    let user_json = pairs
        .iter()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| "Missing user field in Telegram initData".to_string())?;

    #[derive(serde::Deserialize)]
    struct TgUser {
        id: i64,
        first_name: Option<String>,
        last_name: Option<String>,
        username: Option<String>,
    }

    let tg: TgUser = serde_json::from_str(user_json)
        .map_err(|e| format!("Invalid user JSON in initData: {}", e))?;

    Ok(TelegramUser {
        id: tg.id,
        first_name: tg.first_name,
        last_name: tg.last_name,
        username: tg.username,
    })
}

/// Parses a URL-encoded query string into decoded key-value pairs.
fn parse_init_data(s: &str) -> Vec<(String, String)> {
    s.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = percent_decode(parts.next()?);
            let value = percent_decode(parts.next().unwrap_or(""));
            Some((key, value))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(b);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
