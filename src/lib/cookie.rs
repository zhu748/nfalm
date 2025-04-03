use rand::{rng, seq::IteratorRandom};
use std::collections::{HashMap, HashSet};
use tokio::{
    select,
    sync::{mpsc::Receiver, oneshot},
    time::{Instant, Interval},
};
use tracing::{error, info};

use crate::{
    config::{Config, CookieInfo, Reason, UselessCookie},
    error::ClewdrError,
};

pub struct CookieManager {
    valid: HashSet<CookieInfo>,
    dispatched: HashMap<CookieInfo, Instant>,
    exhausted: HashSet<CookieInfo>,
    invalid: HashSet<UselessCookie>,
    req_rx: Receiver<oneshot::Sender<Result<CookieInfo, ClewdrError>>>,
    ret_rx: Receiver<(CookieInfo, Option<Reason>)>,
    config: Config,
    interval: Interval,
}

impl CookieInfo {
    /// check if the cookie is expired
    /// if expired, set the reset time to None
    pub fn reset(self) -> Self {
        if let Some(t) = self.reset_time {
            if t < chrono::Utc::now().timestamp() {
                info!("Cookie reset time expired");
                return Self {
                    reset_time: None,
                    ..self
                };
            }
        }
        self
    }
}

impl CookieManager {
    pub fn new(
        mut config: Config,
        req_rx: Receiver<oneshot::Sender<Result<CookieInfo, ClewdrError>>>,
        ret_rx: Receiver<(CookieInfo, Option<Reason>)>,
    ) -> Self {
        config.cookie_array = config.cookie_array.into_iter().map(|c| c.reset()).collect();
        let valid = HashSet::from_iter(config.cookie_array.iter().filter_map(|c| {
            if c.reset_time.is_none() {
                Some(c.clone())
            } else {
                None
            }
        }));
        let exhaust = HashSet::from_iter(config.cookie_array.iter().filter_map(|c| {
            if c.reset_time.is_some() {
                Some(c.clone())
            } else {
                None
            }
        }));
        let invalid = HashSet::from_iter(config.wasted_cookie.iter().cloned());
        let dispatched = HashMap::new();
        // wait 5 mins to collect unreturned cookies
        let interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
        Self {
            valid,
            exhausted: exhaust,
            invalid,
            req_rx,
            config,
            ret_rx,
            dispatched,
            interval,
        }
    }

    fn log(&self) {
        info!(
            "Valid: {}, Dispatched: {}, Exhausted: {}, Invalid: {}",
            self.valid.len(),
            self.dispatched.len(),
            self.exhausted.len(),
            self.invalid.len()
        );
    }

    fn save(&mut self) {
        let cookies = self
            .valid
            .iter()
            .chain(self.exhausted.iter())
            .chain(self.dispatched.keys())
            .cloned()
            .collect::<Vec<_>>();
        self.config.cookie_array = cookies;
        self.config.wasted_cookie = self.invalid.iter().cloned().collect();
        self.config.save().unwrap_or_else(|e| {
            error!("Failed to save config: {}", e);
        });
    }

    /// Try to dispatch a cookie from the valid set
    fn dispatch(&mut self) -> Result<CookieInfo, ClewdrError> {
        // remove reset cookies
        let exhausted = self
            .exhausted
            .clone()
            .into_iter()
            .map(|c| c.reset())
            .collect::<HashSet<_>>();
        self.valid.extend(
            exhausted
                .clone()
                .into_iter()
                .filter(|c| c.reset_time.is_none()),
        );
        self.exhausted = exhausted
            .into_iter()
            .filter(|c| c.reset_time.is_some())
            .collect();
        self.save();
        // randomly select a cookie from valid cookies and remove it from the set
        let mut rng = rng();
        let cookie = self
            .valid
            .iter()
            .choose(&mut rng)
            .ok_or(ClewdrError::NoCookieAvailable)?
            .clone();
        self.valid.remove(&cookie);
        let instant = Instant::now();
        self.dispatched.insert(cookie.clone(), instant);
        Ok(cookie)
    }

    /// Collect the cookie and update the state
    fn collect(&mut self, mut cookie: CookieInfo, reason: Option<Reason>) {
        if !self.dispatched.contains_key(&cookie) {
            error!("Unknown dispatched");
            return;
        }
        if let Some(reason) = reason {
            match reason {
                Reason::Exhausted(i) => {
                    cookie.reset_time = Some(i);
                    self.exhausted.insert(cookie.clone());
                }
                r => {
                    self.invalid
                        .insert(UselessCookie::new(cookie.cookie.clone(), r));
                }
            }
            self.save();
        } else {
            self.valid.insert(cookie.clone());
        }
        self.dispatched.remove(&cookie);
    }

    /// Run the cookie manager
    /// This function will run in a loop and handle the requests and returns
    /// from the channels
    pub async fn run(mut self) {
        loop {
            self.log();
            select! {
                biased;
                Some((cookie, reason)) = self.ret_rx.recv() => self.collect(cookie, reason),
                _ = self.interval.tick() => {
                    // collect cookies that are not returned for 5 mins
                    let now = Instant::now();
                    let mut to_remove = vec![];
                    for (cookie, instant) in self.dispatched.iter() {
                        if now.duration_since(*instant).as_secs() > 5 * 60 {
                            to_remove.push(cookie.clone());
                        }
                    }
                    for cookie in to_remove {
                        self.valid.insert(cookie.clone());
                        self.dispatched.remove(&cookie);
                    }
                }
                Some(sender) = self.req_rx.recv() => {
                    let cookie = self.dispatch();
                    if let Err(e) = sender.send(cookie) {
                        error!("Failed to send cookie");
                        if let Ok(c) = e {
                            self.valid.insert(c);
                        }
                    }
                }
            }
        }
    }
}
