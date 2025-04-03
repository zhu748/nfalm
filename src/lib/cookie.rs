use rand::{rng, seq::IteratorRandom};
use std::collections::HashSet;
use tokio::{
    select,
    sync::{mpsc::Receiver, oneshot},
};
use tracing::{error, info};

use crate::{
    config::{Config, CookieInfo, Reason, UselessCookie},
    error::ClewdrError,
};

pub struct CookieManager {
    valid: HashSet<CookieInfo>,
    dispatched: HashSet<CookieInfo>,
    exhausted: HashSet<CookieInfo>,
    invalid: HashSet<UselessCookie>,
    req_rx: Receiver<oneshot::Sender<Result<CookieInfo, ClewdrError>>>,
    ret_rx: Receiver<(CookieInfo, Option<Reason>)>,
    config: Config,
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
        let dispatched = HashSet::new();
        Self {
            valid,
            exhausted: exhaust,
            invalid,
            req_rx,
            config,
            ret_rx,
            dispatched,
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
        self.dispatched.insert(cookie.clone());
        Ok(cookie)
    }

    /// Collect the cookie and update the state
    fn collect(&mut self, mut cookie: CookieInfo, reason: Option<Reason>) {
        if !self.dispatched.contains(&cookie) {
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
