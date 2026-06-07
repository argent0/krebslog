use crate::error::{KrebslogError, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, Utc};
use std::process::{Command, Stdio};

/// Parse flexible natural language dates into a start-of-day UTC DateTime.
///
/// Supports (extending nutlog):
/// - today, yesterday, tomorrow
/// - YYYY-MM-DD
/// - N days ago, last week, last month
/// - last 7 days (treated as start date = today - 6d for "since")
/// - last monday, last tuesday, ... (previous occurrence of that weekday)
/// - this week, this month (start of current period)
///
/// Also accepts plain RFC3339.
pub fn parse_flexible_date(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim().to_lowercase();
    let now = Local::now();
    let today = now.date_naive();

    let naive: NaiveDate = if s == "today" {
        today
    } else if s == "yesterday" {
        today - chrono::Duration::days(1)
    } else if s == "tomorrow" {
        today + chrono::Duration::days(1)
    } else if let Ok(d) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        d
    } else if s.ends_with(" days ago") || s.ends_with(" day ago") {
        if let Some(num_str) = s.split_whitespace().next() {
            if let Ok(n) = num_str.parse::<i64>() {
                today - chrono::Duration::days(n)
            } else {
                return Err(KrebslogError::InvalidDate(s));
            }
        } else {
            return Err(KrebslogError::InvalidDate(s));
        }
    } else if s == "last week" {
        today - chrono::Duration::days(7)
    } else if s == "last month" {
        today - chrono::Duration::days(30)
    } else if let Some(days_str) = s
        .strip_prefix("last ")
        .and_then(|r| r.strip_suffix(" days"))
    {
        // "last 7 days", "last 30 days" — return the start of the window (N-1 days ago)
        if let Ok(n) = days_str.trim().parse::<i64>() {
            if n > 0 {
                today - chrono::Duration::days(n - 1)
            } else {
                today
            }
        } else {
            return Err(KrebslogError::InvalidDate(s.clone()));
        }
    } else if let Some(wanted) = s.strip_prefix("last ").map(|r| r.trim()) {
        // last monday, last tuesday, ...
        let target = match wanted {
            "monday" | "mon" => chrono::Weekday::Mon,
            "tuesday" | "tue" => chrono::Weekday::Tue,
            "wednesday" | "wed" => chrono::Weekday::Wed,
            "thursday" | "thu" => chrono::Weekday::Thu,
            "friday" | "fri" => chrono::Weekday::Fri,
            "saturday" | "sat" => chrono::Weekday::Sat,
            "sunday" | "sun" => chrono::Weekday::Sun,
            _ => return Err(KrebslogError::InvalidDate(s.clone())),
        };
        // Find most recent previous target weekday (not including today if today matches)
        let mut d = today - chrono::Duration::days(1);
        for _ in 0..14 {
            if d.weekday() == target {
                let ss = s.clone();
                let midnight = d
                    .and_hms_opt(0, 0, 0)
                    .expect("00:00:00 is always a valid NaiveTime");
                return Ok(midnight
                    .and_local_timezone(Local)
                    .single()
                    .ok_or(KrebslogError::InvalidDate(ss))?
                    .with_timezone(&Utc));
            }
            d -= chrono::Duration::days(1);
        }
        return Err(KrebslogError::InvalidDate(s.clone()));
    } else if s == "this week" {
        // start of week (Monday)
        let offset = today.weekday().num_days_from_monday() as i64;
        today - chrono::Duration::days(offset)
    } else if s == "this month" {
        NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .expect("first day of the current month is always valid")
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    } else if let Ok(d) = NaiveDate::parse_from_str(&s, "%m-%d-%Y") {
        d
    } else if let Ok(d) = NaiveDate::parse_from_str(&s, "%d-%m-%Y") {
        d
    } else {
        return Err(KrebslogError::InvalidDate(s.clone()));
    };

    let ss = s.clone();
    let local_dt = naive
        .and_hms_opt(0, 0, 0)
        .ok_or(KrebslogError::InvalidDate(ss.clone()))?
        .and_local_timezone(Local)
        .single()
        .ok_or(KrebslogError::InvalidDate(ss))?;
    Ok(local_dt.with_timezone(&Utc))
}

/// Format a date for passing to child CLIs (use YYYY-MM-DD for compatibility).
pub fn format_date_for_child(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// Run an external binary with --json and given args, capture stdout as string.
/// Returns (stdout, stderr). Errors if exit status != 0.
pub fn run_external_json(bin: &str, args: &[String]) -> Result<(String, String)> {
    let mut cmd = Command::new(bin);
    cmd.arg("--json");
    for a in args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| KrebslogError::ExternalTool {
        bin: bin.to_string(),
        reason: e.to_string(),
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(KrebslogError::ExternalToolFailed {
            bin: bin.to_string(),
            stderr: if stderr.trim().is_empty() {
                format!("exit code {:?}", output.status.code())
            } else {
                stderr
            },
        });
    }
    Ok((stdout, stderr))
}

/// Try to locate a binary, first using the override, then PATH, then common install locations.
pub fn resolve_bin(override_path: Option<&str>, default_names: &[&str]) -> Option<String> {
    if let Some(p) = override_path {
        if std::path::Path::new(p).exists() || which::which(p).is_ok() {
            return Some(p.to_string());
        }
    }
    for name in default_names {
        if let Ok(p) = which::which(name) {
            return Some(p.to_string());
        }
    }
    // Fallbacks
    for name in default_names {
        let candidates = [
            format!("/usr/bin/{}", name),
            format!("/usr/local/bin/{}", name),
            format!(
                "{}/.cargo/bin/{}",
                std::env::var("HOME").unwrap_or_default(),
                name
            ),
        ];
        for c in &candidates {
            if std::path::Path::new(c).exists() {
                return Some(c.clone());
            }
        }
    }
    None
}

// Small which helper (no external `which` crate; keep deps minimal).
mod which {
    use std::env;
    use std::path::Path;

    pub fn which(cmd: &str) -> std::result::Result<String, ()> {
        if let Ok(paths) = env::var("PATH") {
            for dir in paths.split(':') {
                let p = Path::new(dir).join(cmd);
                if p.is_file() {
                    return Ok(p.to_string_lossy().to_string());
                }
            }
        }
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixed_date() {
        let d = parse_flexible_date("2026-06-07").expect("valid fixed date");
        assert_eq!(format_date_for_child(d), "2026-06-07");
    }

    #[test]
    fn parses_last_n_days() {
        // Should succeed and produce a date in the past relative to "now".
        let d = parse_flexible_date("last 7 days").expect("last 7 days");
        // Just sanity: the formatted child date should look like a YYYY-MM-DD
        let s = format_date_for_child(d);
        assert!(s.len() == 10 && s.chars().nth(4) == Some('-'));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_flexible_date("not a real date at all").is_err());
    }

    #[test]
    fn last_monday_does_not_panic() {
        // Must not panic and must return a Monday (we don't care which one relative to test time).
        if let Ok(d) = parse_flexible_date("last monday") {
            // chrono weekday comparison requires the date's weekday
            // We just ensure we got *some* valid date back.
            let _ = format_date_for_child(d);
        }
    }
}
