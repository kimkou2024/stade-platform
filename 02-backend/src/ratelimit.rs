//! Lightweight in-memory rate limiting [M §10.3]: a per-key sliding-window
//! request cap (anti-abuse) and a per-email failed-login counter
//! (anti-brute-force). Self-contained; no external middleware dependency.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct RlState {
    reqs: HashMap<String, Vec<Instant>>,
    fails: HashMap<String, (u32, Instant)>,
}

pub struct Limits {
    pub window: Duration,     // request window
    pub max: usize,           // max requests per window per key
    pub fail_window: Duration,
    pub fail_max: u32,        // failed logins before lockout
}

impl RlState {
    /// Returns false when the key has exceeded its request budget.
    pub fn allow(&mut self, key: &str, lim: &Limits, now: Instant) -> bool {
        let v = self.reqs.entry(key.to_string()).or_default();
        v.retain(|t| now.duration_since(*t) < lim.window);
        if v.len() >= lim.max { return false; }
        v.push(now);
        true
    }

    pub fn record_fail(&mut self, email: &str, lim: &Limits, now: Instant) {
        let e = self.fails.entry(email.to_string()).or_insert((0, now));
        if now.duration_since(e.1) > lim.fail_window { *e = (0, now); }
        e.0 += 1; e.1 = now;
    }

    pub fn too_many_fails(&self, email: &str, lim: &Limits, now: Instant) -> bool {
        match self.fails.get(email) {
            Some((n, t)) => *n >= lim.fail_max && now.duration_since(*t) < lim.fail_window,
            None => false,
        }
    }

    pub fn clear_fails(&mut self, email: &str) { self.fails.remove(email); }
}

pub type SharedRl = std::sync::Arc<Mutex<RlState>>;
