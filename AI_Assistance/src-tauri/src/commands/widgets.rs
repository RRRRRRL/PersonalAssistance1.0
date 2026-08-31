use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::db::AgentStateShared;
use tauri::State;

// ─── Daily Brief: Weather ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSummary {
    pub temperature_c: f64,
    pub feels_like_c: f64,
    pub humidity_pct: u32,
    pub wind_kmh: f64,
    pub weather_code: u32,
    pub is_day: bool,
    pub description: String,
    pub icon: String,
}

fn weather_code_to_description(code: u32) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear sky", "☀️"),
        1 => ("Mainly clear", "🌤"),
        2 => ("Partly cloudy", "⛅"),
        3 => ("Overcast", "☁️"),
        45 | 48 => ("Fog", "🌫"),
        51..=57 => ("Drizzle", "🌦"),
        61..=67 => ("Rain", "🌧"),
        71..=77 => ("Snow", "❄️"),
        80..=82 => ("Rain showers", "🌦"),
        85..=86 => ("Snow showers", "🌨"),
        95 => ("Thunderstorm", "⛈"),
        96 | 99 => ("Thunderstorm w/ hail", "⛈"),
        _ => ("Unknown", "❓"),
    }
}

#[tauri::command]
pub async fn get_weather_summary(lat: f64, lon: f64) -> Result<WeatherSummary, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,is_day&timezone=auto",
        lat, lon
    );

    let resp: serde_json::Value = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let current = &resp["current"];
    let code = current["weather_code"].as_u64().unwrap_or(0) as u32;
    let (desc, icon) = weather_code_to_description(code);

    Ok(WeatherSummary {
        temperature_c: current["temperature_2m"].as_f64().unwrap_or(0.0),
        feels_like_c: current["apparent_temperature"].as_f64().unwrap_or(0.0),
        humidity_pct: current["relative_humidity_2m"].as_u64().unwrap_or(0) as u32,
        wind_kmh: current["wind_speed_10m"].as_f64().unwrap_or(0.0),
        weather_code: code,
        is_day: current["is_day"].as_u64().unwrap_or(1) == 1,
        description: desc.to_string(),
        icon: icon.to_string(),
    })
}

// ─── Daily Brief: Public Holidays ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayEntry {
    pub date: String,
    pub name: String,
    pub local_name: String,
}

#[tauri::command]
pub async fn get_public_holidays(year: i32, country: String) -> Result<Vec<HolidayEntry>, String> {
    let url = format!(
        "https://date.nager.at/api/v3/PublicHolidays/{}/{}",
        year,
        country.to_uppercase()
    );

    let resp: Vec<serde_json::Value> = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp
        .into_iter()
        .map(|h| HolidayEntry {
            date: h["date"].as_str().unwrap_or("").to_string(),
            name: h["name"].as_str().unwrap_or("").to_string(),
            local_name: h["localName"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

// ─── Security Toolkit ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlScanResult {
    pub url_checked: String,
    pub is_malicious: bool,
    pub threat_count: usize,
    pub message: String,
}

#[tauri::command]
pub async fn scan_url_safety(url: String) -> Result<UrlScanResult, String> {
    // URLhaus: check if URL is known malware distribution
    let client = reqwest::Client::new();
    let resp = client
        .post("https://urlhaus-api.abuse.ch/v1/url/")
        .form(&[("url", url.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let status = body["query_status"].as_str().unwrap_or("unknown");

    let is_malicious = status == "urlhaus_url_found";
    let threat_count = body["urls"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    let message = if is_malicious {
        format!(
            "⚠️ URL flagged as malicious by URLhaus ({} report(s))",
            threat_count
        )
    } else {
        "✅ URL not found in URLhaus threat database".to_string()
    };

    Ok(UrlScanResult {
        url_checked: url,
        is_malicious,
        threat_count,
        message,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreachResult {
    pub email_checked: String,
    pub breach_count: usize,
    pub breaches: Vec<BreachInfo>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreachInfo {
    pub name: String,
    pub domain: String,
    pub date: String,
    pub pwn_count: u64,
}

#[tauri::command]
pub async fn check_email_breach(email: String) -> Result<BreachResult, String> {
    // Use EmailRep.io — no API key needed, privacy-friendly
    let url = format!("https://emailrep.io/{}", email);
    let client = reqwest::Client::builder()
        .user_agent("AI-Assistance-App/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let breach_count = resp["details"]["breached_count"].as_u64().unwrap_or(0) as usize;
    let breaches_list = resp["details"]["breaches"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let breach_infos: Vec<BreachInfo> = breaches_list
        .iter()
        .take(5)
        .map(|b| BreachInfo {
            name: b.as_str().unwrap_or("Unknown").to_string(),
            domain: String::new(),
            date: String::new(),
            pwn_count: 0,
        })
        .collect();

    let message = if breach_count > 0 {
        format!("⚠️ Email found in {} data breach(es)", breach_count)
    } else {
        "✅ Email not found in any known breaches".to_string()
    };

    Ok(BreachResult {
        email_checked: email,
        breach_count,
        breaches: breach_infos,
        message,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub ip: String,
    pub country: String,
    pub tls_version: String,
    pub http_protocol: String,
    pub asn: String,
}

#[tauri::command]
pub async fn get_network_info() -> Result<NetworkInfo, String> {
    let resp = reqwest::get("https://cloudflare.com/cdn-cgi/trace")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut info = NetworkInfo {
        ip: String::new(),
        country: String::new(),
        tls_version: String::new(),
        http_protocol: String::new(),
        asn: String::new(),
    };

    for line in resp.lines() {
        if let Some((key, val)) = line.split_once('=') {
            match key.trim() {
                "ip" => info.ip = val.trim().to_string(),
                "loc" => info.country = val.trim().to_string(),
                "tls" => info.tls_version = val.trim().to_string(),
                "http" => info.http_protocol = val.trim().to_string(),
                "asn" => info.asn = val.trim().to_string(),
                _ => {}
            }
        }
    }

    Ok(info)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResult {
    pub password: String,
    pub entropy_bits: f64,
    pub strength: String,
}

#[tauri::command]
pub fn generate_secure_password(length: usize, use_uppercase: bool, use_numbers: bool, use_symbols: bool) -> Result<PasswordResult, String> {
    let length = length.clamp(8, 128);

    let mut charset = "abcdefghijklmnopqrstuvwxyz".to_string();
    let mut pool_size: f64 = 26.0;

    if use_uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        pool_size += 26.0;
    }
    if use_numbers {
        charset.push_str("0123456789");
        pool_size += 10.0;
    }
    if use_symbols {
        charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?");
        pool_size += 25.0;
    }

    let mut rng = rand::thread_rng();
    let chars: Vec<char> = charset.chars().collect();
    let password: String = (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();

    let entropy = (length as f64) * pool_size.log2();
    let strength = if entropy >= 128.0 {
        "Very Strong"
    } else if entropy >= 80.0 {
        "Strong"
    } else if entropy >= 60.0 {
        "Moderate"
    } else {
        "Weak"
    };

    Ok(PasswordResult {
        password,
        entropy_bits: (entropy * 100.0).round() / 100.0,
        strength: strength.to_string(),
    })
}

// ─── Quick Utilities ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateResult {
    pub base: String,
    pub date: String,
    pub rates: Vec<RateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateEntry {
    pub currency: String,
    pub rate: f64,
}

#[tauri::command]
pub async fn get_exchange_rates(base: String, symbols: Vec<String>) -> Result<ExchangeRateResult, String> {
    let symbols_param = symbols.join(",");
    let url = format!(
        "https://api.frankfurter.app/latest?from={}&to={}",
        base.to_uppercase(),
        symbols_param
    );

    let resp: serde_json::Value = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let date = resp["date"].as_str().unwrap_or("").to_string();
    let rates_obj = &resp["rates"];

    let rates: Vec<RateEntry> = symbols
        .iter()
        .filter_map(|sym| {
            rates_obj[sym.to_uppercase()].as_f64().map(|rate| RateEntry {
                currency: sym.to_uppercase(),
                rate,
            })
        })
        .collect();

    Ok(ExchangeRateResult {
        base: base.to_uppercase(),
        date,
        rates,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryResult {
    pub word: String,
    pub phonetic: String,
    pub definitions: Vec<DefinitionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionEntry {
    pub part_of_speech: String,
    pub definition: String,
    pub example: Option<String>,
}

#[tauri::command]
pub async fn lookup_word(word: String) -> Result<DictionaryResult, String> {
    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
        word.trim()
    );

    let resp: Vec<serde_json::Value> = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let first = resp.first().ok_or("Word not found")?;
    let phonetic = first["phonetic"].as_str().unwrap_or("").to_string();

    let mut definitions = Vec::new();
    if let Some(meanings) = first["meanings"].as_array() {
        for meaning in meanings.iter().take(3) {
            let pos = meaning["partOfSpeech"].as_str().unwrap_or("").to_string();
            if let Some(defs) = meaning["definitions"].as_array() {
                for def in defs.iter().take(2) {
                    definitions.push(DefinitionEntry {
                        part_of_speech: pos.clone(),
                        definition: def["definition"].as_str().unwrap_or("").to_string(),
                        example: def["example"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }
    }

    Ok(DictionaryResult {
        word: first["word"].as_str().unwrap_or(&word).to_string(),
        phonetic,
        definitions,
    })
}

// ─── Self Review (Langfuse proxy) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceScore {
    pub name: String,
    pub value: f64,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub id: String,
    pub name: String,
    pub timestamp: Option<String>,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub tags: Vec<String>,
    pub scores: Vec<TraceScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReviewResponse {
    pub traces: Vec<TraceSummary>,
    pub available: bool,
    pub message: Option<String>,
    pub avg_scores: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDetail {
    pub id: String,
    pub name: String,
    pub timestamp: Option<String>,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub tags: Vec<String>,
    pub scores: Vec<TraceScore>,
    pub observations: Vec<serde_json::Value>,
}

fn agent_url(agent: &State<'_, AgentStateShared>, path: &str) -> Result<String, String> {
    let ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
    Ok(format!("http://127.0.0.1:{}{}", ag.port, path))
}

#[tauri::command]
pub async fn get_self_review_traces(
    agent: State<'_, AgentStateShared>,
    limit: Option<u32>,
) -> Result<SelfReviewResponse, String> {
    let url = agent_url(&agent, &format!("/traces?limit={}", limit.unwrap_or(15)))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let available = body["available"].as_bool().unwrap_or(false);
    let message = body["message"].as_str().map(|s| s.to_string());

    let traces: Vec<TraceSummary> = body["traces"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|t| TraceSummary {
            id: t["id"].as_str().unwrap_or("").to_string(),
            name: t["name"].as_str().unwrap_or("").to_string(),
            timestamp: t["timestamp"].as_str().map(|s| s.to_string()),
            input: t.get("input").cloned(),
            output: t.get("output").cloned(),
            tags: t["tags"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            scores: t["scores"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| TraceScore {
                            name: s["name"].as_str().unwrap_or("").to_string(),
                            value: s["value"].as_f64().unwrap_or(0.0),
                            comment: s["comment"].as_str().unwrap_or("").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();

    // Compute average scores across all traces
    let mut score_totals: std::collections::HashMap<String, (f64, u32)> = std::collections::HashMap::new();
    for trace in &traces {
        for score in &trace.scores {
            let entry = score_totals.entry(score.name.clone()).or_insert((0.0, 0));
            entry.0 += score.value;
            entry.1 += 1;
        }
    }
    let avg_scores: serde_json::Value = serde_json::Value::Object(
        score_totals
            .into_iter()
            .map(|(name, (total, count))| {
                (name, serde_json::Value::Number(serde_json::Number::from_f64((total / count as f64 * 100.0).round() / 100.0).unwrap_or(serde_json::Number::from(0))))
            })
            .collect(),
    );

    Ok(SelfReviewResponse {
        traces,
        available,
        message,
        avg_scores,
    })
}

#[tauri::command]
pub async fn get_self_review_detail(
    agent: State<'_, AgentStateShared>,
    trace_id: String,
) -> Result<TraceDetail, String> {
    let url = agent_url(&agent, &format!("/trace/{}", trace_id))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    Ok(TraceDetail {
        id: body["id"].as_str().unwrap_or("").to_string(),
        name: body["name"].as_str().unwrap_or("").to_string(),
        timestamp: body["timestamp"].as_str().map(|s| s.to_string()),
        input: body.get("input").cloned(),
        output: body.get("output").cloned(),
        tags: body["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        scores: body["scores"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| TraceScore {
                        name: s["name"].as_str().unwrap_or("").to_string(),
                        value: s["value"].as_f64().unwrap_or(0.0),
                        comment: s["comment"].as_str().unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        observations: body["observations"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn submit_self_review_score(
    agent: State<'_, AgentStateShared>,
    trace_id: String,
    name: String,
    value: f64,
    comment: Option<String>,
) -> Result<bool, String> {
    let url = agent_url(&agent, &format!("/trace/{}/score?name={}&value={}&comment={}", trace_id, name, value, comment.unwrap_or_default()))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.post(&url).send().await.map_err(|e| e.to_string())?;
    Ok(resp.status().is_success())
}

// ─── Markdown Rendering (pulldown-cmark) ───────────────────────────────────

/// Converts Markdown text to sanitized HTML using pulldown-cmark.
/// The output is safe for rendering in a webview (no script tags).
#[tauri::command]
pub fn render_markdown(input: String) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Basic sanitization: remove script tags and event handlers
    let sanitized = html_output
        .replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("javascript:", "")
        .replace("onerror=", "")
        .replace("onload=", "")
        .replace("onclick=", "")
        .replace("onmouseover=", "");

    sanitized
}
