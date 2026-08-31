use chrono::Local;
use regex::{Captures, Regex};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
pub struct SensitiveScanResult {
    pub redacted_text: String,
    pub has_sensitive: bool,
    pub redaction_count: i64,
    pub detections: Vec<String>,
    pub severity: String,
}

fn now_ts() -> i64 {
    Local::now().timestamp()
}

fn luhn_check(card_digits: &str) -> bool {
    if card_digits.len() < 13 || card_digits.len() > 19 {
        return false;
    }
    let mut sum = 0;
    let mut alt = false;
    for ch in card_digits.chars().rev() {
        let mut digit = match ch.to_digit(10) {
            Some(v) => v,
            None => return false,
        };
        if alt {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
        alt = !alt;
    }
    sum % 10 == 0
}

/// Apply a single regex redaction pass. Returns updated text.
fn redact<F>(text: &str, pattern: &str, detection_label: &str,
             detections: &mut BTreeSet<String>, count: &mut i64, replacer: F) -> String
where
    F: Fn(&Captures) -> String,
{
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };
    re.replace_all(text, |caps: &Captures| {
        *count += 1;
        detections.insert(detection_label.to_string());
        replacer(caps)
    })
    .to_string()
}

pub fn scan_and_redact(input: &str) -> Result<SensitiveScanResult, String> {
    let mut text = input.to_string();
    let mut detections = BTreeSet::new();
    let mut redaction_count: i64 = 0;

    // ── 1. Private key blocks ──────────────────────────────────────────────────
    text = redact(&text,
        r"(?s)-----BEGIN\s[A-Z ]*PRIVATE\sKEY-----.+?-----END\s[A-Z ]*PRIVATE\sKEY-----",
        "private_key", &mut detections, &mut redaction_count,
        |_| "[REDACTED_PRIVATE_KEY]".to_string());

    // ── 2. AWS access key IDs (AKIA...) ────────────────────────────────────────
    text = redact(&text,
        r"\bAKIA[0-9A-Z]{16}\b",
        "aws_access_key", &mut detections, &mut redaction_count,
        |_| "[REDACTED_AWS_KEY]".to_string());

    // ── 3. Generic API keys / tokens ───────────────────────────────────────────
    //    sk-*, sk_live_*, sk_test_*, ghp_*, gho_*, ghu_*, ghs_*, ghr_*,
    //    xoxb-*, xoxp-*, xapp-*, Bearer eyJ..., hf_*, dhp_*
    text = redact(&text,
        r"(?i)\b(?:sk[-_][a-zA-Z0-9]{20,}|gh[poushr]_[A-Za-z0-9]{36,}|xox[baprs]-[A-Za-z0-9\-]{10,}|xapp-[A-Za-z0-9\-]{10,}|hf_[A-Za-z0-9]{34,}|dhp_[A-Za-z0-9]{30,})\b",
        "api_key_or_token", &mut detections, &mut redaction_count,
        |_| "[REDACTED_API_KEY]".to_string());

    // ── 4. Bearer / Basic auth headers ─────────────────────────────────────────
    text = redact(&text,
        r"(?i)(authorization\s*:\s*(?:bearer|basic|token))\s+\S+",
        "auth_header", &mut detections, &mut redaction_count,
        |caps| format!("{} [REDACTED_AUTH_VALUE]",
            caps.get(1).map(|m| m.as_str()).unwrap_or("authorization")) );

    // ── 5. JWT tokens (eyJ...) ─────────────────────────────────────────────────
    text = redact(&text,
        r"\bey[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
        "jwt_token", &mut detections, &mut redaction_count,
        |_| "[REDACTED_JWT]".to_string());

    // ── 6. Passwords ───────────────────────────────────────────────────────────
    text = redact(&text,
        r"(?i)\b(password|passwd|pwd)\b\s*[:=]\s*\S+",
        "password", &mut detections, &mut redaction_count,
        |caps| format!("{}=[REDACTED_PASSWORD]",
            caps.get(1).map(|m| m.as_str()).unwrap_or("password")) );

    // ── 7. OTP / 2FA codes ─────────────────────────────────────────────────────
    text = redact(&text,
        r"(?i)\b(?:otp|2fa|verification|passcode|auth(?:entication)?\s*code)\b[^\n]{0,30}\b\d{4,8}\b",
        "otp_or_2fa", &mut detections, &mut redaction_count,
        |caps| {
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            Regex::new(r"\d{4,8}").expect("valid regex")
                .replace(full, "[REDACTED_OTP]").to_string()
        });

    // ── 8. CVV / CVC ───────────────────────────────────────────────────────────
    text = redact(&text,
        r"(?i)\b(cvv|cvc)\b\s*[:=]?\s*\d{3,4}\b",
        "cvv", &mut detections, &mut redaction_count,
        |caps| format!("{} [REDACTED_CVV]",
            caps.get(1).map(|m| m.as_str()).unwrap_or("cvv")) );

    // ── 9. SSN / national ID ───────────────────────────────────────────────────
    text = redact(&text,
        r"\b\d{3}-\d{2}-\d{4}\b",
        "national_id_or_ssn", &mut detections, &mut redaction_count,
        |_| "[REDACTED_SSN]".to_string());

    // ── 10. Bank account / routing numbers ─────────────────────────────────────
    text = redact(&text,
        r"(?i)\b(account|acct|routing)\b[^\n]{0,20}\b\d{6,17}\b",
        "bank_account_or_routing", &mut detections, &mut redaction_count,
        |caps| {
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            Regex::new(r"\d{6,17}").expect("valid regex")
                .replace(full, "[REDACTED_ACCOUNT]").to_string()
        });

    // ── 11. Credit / debit card numbers (Luhn-validated) ──────────────────────
    text = redact(&text,
        r"\b(?:\d[ -]*?){13,19}\b",
        "__card_candidate__", &mut detections, &mut redaction_count,
        |caps| {
            let candidate = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let digits = candidate.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
            if luhn_check(&digits) {
                detections.remove("__card_candidate__");
                detections.insert("card_number".to_string());
                "[REDACTED_CARD]".to_string()
            } else {
                candidate.to_string()
            }
        });
    detections.remove("__card_candidate__");

    // ── 12. Email addresses ────────────────────────────────────────────────────
    text = redact(&text,
        r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b",
        "email_address", &mut detections, &mut redaction_count,
        |_| "[REDACTED_EMAIL]".to_string());

    // ── 13. Phone numbers (international & US formats) ─────────────────────────
    text = redact(&text,
        r"(?:(?:\+\d{1,3}[\s\-]?)?\(?\d{3}\)?[\s\-]?\d{3}[\s\-]?\d{4})\b",
        "phone_number", &mut detections, &mut redaction_count,
        |_| "[REDACTED_PHONE]".to_string());

    // ── 14. IPv4 addresses (excluding 0.0.0.0 and 255.x) ──────────────────────
    text = redact(&text,
        r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b",
        "ip_address", &mut detections, &mut redaction_count,
        |caps| {
            let ip = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if ip.starts_with("0.") || ip.starts_with("255.") || ip == "127.0.0.1" {
                ip.to_string()
            } else {
                "[REDACTED_IP]".to_string()
            }
        });

    // ── 15. Connection strings / DSN ───────────────────────────────────────────
    text = redact(&text,
        r"(?i)(mongodb|postgres|mysql|redis|amqp|smtp)(://\S+)",
        "connection_string", &mut detections, &mut redaction_count,
        |caps| format!("{}:[REDACTED_DSN]",
            caps.get(1).map(|m| m.as_str()).unwrap_or("service")) );

    // ── Severity calculation ───────────────────────────────────────────────────
    let critical_labels = ["private_key", "aws_access_key", "api_key_or_token",
                           "auth_header", "jwt_token", "password", "connection_string"];
    let high_labels = ["card_number", "national_id_or_ssn", "bank_account_or_routing",
                       "cvv", "otp_or_2fa"];

    let severity = if detections.iter().any(|d| critical_labels.contains(&d.as_str())) {
        "critical"
    } else if detections.iter().any(|d| high_labels.contains(&d.as_str())) {
        "high"
    } else if redaction_count > 0 {
        "medium"
    } else {
        "none"
    };

    Ok(SensitiveScanResult {
        redacted_text: text,
        has_sensitive: redaction_count > 0,
        redaction_count,
        detections: detections.into_iter().collect::<Vec<_>>(),
        severity: severity.to_string(),
    })
}

pub fn log_redaction_event(
    conn: &Connection,
    source: &str,
    result: &SensitiveScanResult,
) -> Result<(), String> {
    if !result.has_sensitive {
        return Ok(());
    }
    let detection_types = result.detections.join(",");
    conn.execute(
        "INSERT INTO sensitive_redaction_events
         (source, redaction_count, detection_types, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![source, result.redaction_count, detection_types, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn inspect_sensitive_text(text: String) -> Result<SensitiveScanResult, String> {
    scan_and_redact(&text)
}
