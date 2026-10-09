//! Shared request pacing and server-directed cooldowns. Cache reads bypass this
//! gate; only background network workers acquire a permit.
use reqwest::{blocking::Response, header::RETRY_AFTER};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Default)]
struct State {
    last_start: Option<Instant>,
    active: usize,
    blocked_until: u64,
    strikes: u32,
}

struct Gate {
    state: Mutex<State>,
    changed: Condvar,
}

impl Gate {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        }
    }

    fn acquire(&self) -> Result<Permit<'_>, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let remaining = state.blocked_until.saturating_sub(crate::storage::now());
            if remaining > 0 {
                return Err(cooldown_message(remaining));
            }
            let delay = state.last_start.map_or(Duration::ZERO, |last| {
                Duration::from_millis(300).saturating_sub(last.elapsed())
            });
            if state.active < 2 && delay.is_zero() {
                state.active += 1;
                state.last_start = Some(Instant::now());
                return Ok(Permit { gate: self });
            }
            let wait = if delay.is_zero() {
                Duration::from_millis(300)
            } else {
                delay
            };
            state = self
                .changed
                .wait_timeout(state, wait)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn observe(&self, status: u16, retry_after: Option<&str>, now: u64) -> Result<(), String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if status == 429 {
            state.strikes = state.strikes.saturating_add(1);
            let fallback = (30_u64 * (1 << state.strikes.saturating_sub(1).min(4))).min(600);
            let delay = retry_after
                .and_then(|value| retry_seconds(value, now))
                .unwrap_or(fallback)
                .max(1);
            state.blocked_until = state.blocked_until.max(now.saturating_add(delay));
            self.changed.notify_all();
            return Err(cooldown_message(state.blocked_until.saturating_sub(now)));
        }
        if (200..300).contains(&status) && state.blocked_until <= now {
            state.strikes = 0;
        }
        Ok(())
    }
}

pub struct Permit<'a> {
    gate: &'a Gate,
}

impl Permit<'_> {
    pub fn check(&self, response: &Response) -> Result<(), String> {
        self.gate.observe(
            response.status().as_u16(),
            response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok()),
            crate::storage::now(),
        )
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active = state.active.saturating_sub(1);
        self.gate.changed.notify_all();
    }
}

fn gate(url: &str) -> &'static Gate {
    static WHAKOOM: OnceLock<Gate> = OnceLock::new();
    static MANGA: OnceLock<Gate> = OnceLock::new();
    let manga = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .is_some_and(|host| {
            matches!(
                host.as_str(),
                "www.listadomanga.es" | "listadomanga.es" | "static.listadomanga.com"
            )
        });
    if manga {
        MANGA.get_or_init(Gate::new)
    } else {
        WHAKOOM.get_or_init(Gate::new)
    }
}

pub fn before(url: &str) -> Result<Permit<'static>, String> {
    gate(url).acquire()
}

pub fn paused(url: &str) -> bool {
    gate(url)
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .blocked_until
        > crate::storage::now()
}

fn cooldown_message(seconds: u64) -> String {
    format!(
        "HTTP 429: el servidor pidió una pausa. Reintentá en {seconds} s; los datos guardados siguen disponibles."
    )
}

fn retry_seconds(value: &str, now: u64) -> Option<u64> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return value.parse().ok();
    }
    // IMF-fixdate, the HTTP-date format used by Retry-After.
    let fields: Vec<_> = value.split_whitespace().collect();
    if fields.len() != 6 || fields[5] != "GMT" {
        return None;
    }
    let day = fields[1].parse::<u32>().ok()?;
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|name| *name == fields[2])? as u32
        + 1;
    let year = fields[3].parse::<i32>().ok()?;
    if !(1970..=9999).contains(&year) || day == 0 || day > crate::calendar::days(year, month) {
        return None;
    }
    let time: Vec<u64> = fields[4]
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if time.len() != 3 || time[0] > 23 || time[1] > 59 || time[2] > 59 {
        return None;
    }
    let days = (1970..year)
        .map(|y| {
            if crate::calendar::days(y, 2) == 29 {
                366_u64
            } else {
                365
            }
        })
        .sum::<u64>()
        + (1..month)
            .map(|m| crate::calendar::days(year, m) as u64)
            .sum::<u64>()
        + day as u64
        - 1;
    Some((days * 86400 + time[0] * 3600 + time[1] * 60 + time[2]).saturating_sub(now))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_after_supports_seconds_and_http_dates_and_rejects_invalid_values() {
        assert_eq!(retry_seconds("120", 0), Some(120));
        assert_eq!(retry_seconds("Thu, 01 Jan 1970 00:02:00 GMT", 30), Some(90));
        assert_eq!(retry_seconds("Thu, 01 Jan 1970 00:02:00 GMT", 150), Some(0));
        assert_eq!(retry_seconds("Wed, 31 Feb 2024 00:00:00 GMT", 0), None);
        assert_eq!(retry_seconds("-1", 0), None);
    }
    #[test]
    fn rate_limit_pauses_other_workers_without_sending_and_success_cannot_shorten_it() {
        let gate = Gate::new();
        let now = crate::storage::now();
        assert!(gate.observe(429, Some("120"), now).is_err());
        assert!(gate.acquire().is_err());
        gate.observe(200, None, now + 1).unwrap();
        assert_eq!(gate.state.lock().unwrap().blocked_until, now + 120);
        assert!(gate.observe(429, Some("1"), now + 2).is_err());
        assert_eq!(gate.state.lock().unwrap().blocked_until, now + 120);
        assert!(gate.observe(429, None, now + 200).is_err());
        assert_eq!(gate.state.lock().unwrap().blocked_until, now + 320);
    }
}
