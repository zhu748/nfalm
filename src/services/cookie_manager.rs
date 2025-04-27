use colored::Colorize;
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tokio::{
    spawn,
    sync::{Mutex, Notify},
    sync::{mpsc, oneshot},
    time::{Instant, Interval},
};
use tracing::{error, info, warn};

use crate::{
    config::{CLEWDR_CONFIG, ClewdrConfig, CookieStatus, Reason, UselessCookie},
    error::ClewdrError,
};

const INTERVAL: u64 = 300;

#[derive(Debug, Serialize, Clone)]
pub struct CookieStatusInfo {
    pub valid: Vec<CookieStatus>,
    pub dispatched: Vec<(CookieStatus, u64)>,
    pub exhausted: Vec<CookieStatus>,
    pub invalid: Vec<UselessCookie>,
}

/// Unified event enum for cookie management with built-in priority ordering
#[derive(Debug)]
pub enum CookieEvent {
    /// Return a Cookie
    Return(CookieStatus, Option<Reason>),
    /// Submit a new Cookie
    Submit(CookieStatus),
    /// Check for timed out Cookies
    CheckTimeout,
    /// Request to get a Cookie
    Request(oneshot::Sender<Result<CookieStatus, ClewdrError>>),
    /// Get all Cookie status information
    GetStatus(oneshot::Sender<CookieStatusInfo>),
    /// Delete a Cookie
    Delete(CookieStatus, oneshot::Sender<Result<(), ClewdrError>>),
}

/// Implements comparison trait for CookieEvent, used for priority ordering
impl PartialEq for CookieEvent {
    fn eq(&self, other: &Self) -> bool {
        self.priority_value() == other.priority_value()
    }
}

impl Eq for CookieEvent {}

impl PartialOrd for CookieEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CookieEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note: We return reverse ordering, so smaller numbers have higher priority
        other.priority_value().cmp(&self.priority_value())
    }
}

impl CookieEvent {
    /// Gets the priority value of the event
    ///
    /// # Returns
    /// * `u8` - The priority value (lower is higher priority)
    fn priority_value(&self) -> u8 {
        match self {
            CookieEvent::Return(_, _) => 0, // Highest priority
            CookieEvent::Submit(_) => 1,
            CookieEvent::Delete(_, _) => 2,
            CookieEvent::GetStatus(_) => 3,
            CookieEvent::CheckTimeout => 4,
            CookieEvent::Request(_) => 5, // Lowest priority
        }
    }
}

/// Cookie manager that handles cookie distribution, collection, and status tracking
pub struct CookieManager {
    valid: VecDeque<CookieStatus>,
    dispatched: HashMap<CookieStatus, Instant>,
    exhausted: HashSet<CookieStatus>,
    invalid: HashSet<UselessCookie>,
    event_queue: Arc<Mutex<BinaryHeap<CookieEvent>>>,
    event_notify: Arc<Notify>, // Notification mechanism
}

/// Event sender interface provided for external components to interact with the cookie manager
#[derive(Clone)]
pub struct CookieEventSender {
    sender: mpsc::Sender<CookieEvent>,
}

impl CookieEventSender {
    /// Request a cookie from the cookie manager
    ///
    /// # Returns
    /// * `Result<CookieStatus, ClewdrError>` - Cookie if available, error otherwise
    pub async fn request(&self) -> Result<CookieStatus, ClewdrError> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CookieEvent::Request(tx)).await?;
        rx.await?
    }

    /// Return a cookie to the cookie manager with optional reason
    ///
    /// # Arguments
    /// * `cookie` - The cookie to return
    /// * `reason` - Optional reason for returning the cookie (e.g., invalid, restricted)
    ///
    /// # Returns
    /// Result indicating success or send error
    pub async fn return_cookie(
        &self,
        cookie: CookieStatus,
        reason: Option<Reason>,
    ) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::Return(cookie, reason)).await
    }

    /// Submit a new cookie to the cookie manager
    ///
    /// # Arguments
    /// * `cookie` - The new cookie to add
    ///
    /// # Returns
    /// Result indicating success or send error
    pub async fn submit(
        &self,
        cookie: CookieStatus,
    ) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::Submit(cookie)).await
    }

    /// Get status information about all cookies
    ///
    /// # Returns
    /// * `Result<CookieStatusInfo, ClewdrError>` - Status information about all cookies
    pub async fn get_status(&self) -> Result<CookieStatusInfo, ClewdrError> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CookieEvent::GetStatus(tx)).await?;
        Ok(rx.await?)
    }

    /// Delete a cookie from the cookie manager
    ///
    /// # Arguments
    /// * `cookie` - The cookie to delete
    ///
    /// # Returns
    /// * `Result<(), ClewdrError>` - Success or error
    pub async fn delete_cookie(&self, cookie: CookieStatus) -> Result<(), ClewdrError> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CookieEvent::Delete(cookie, tx)).await?;
        rx.await?
    }

    /// Used for internal timeout checking
    /// Sends a timeout check event to the cookie manager
    ///
    /// # Returns
    /// Result indicating success or send error
    pub(crate) async fn check_timeout(&self) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::CheckTimeout).await
    }
}

impl CookieManager {
    /// Starts the cookie manager and returns an event sender
    ///
    /// Initializes cookie collections, creates event channels and queues,
    /// and spawns the event processing task
    ///
    /// # Returns
    /// * `CookieEventSender` - Event sender for interacting with the cookie manager
    pub fn start() -> CookieEventSender {
        let valid = VecDeque::from_iter(
            CLEWDR_CONFIG
                .load()
                .cookie_array
                .iter()
                .filter(|c| c.reset_time.is_none())
                .cloned(),
        );
        let exhaust = HashSet::from_iter(
            CLEWDR_CONFIG
                .load()
                .cookie_array
                .iter()
                .filter(|c| c.reset_time.is_some())
                .cloned(),
        );
        let invalid = HashSet::from_iter(CLEWDR_CONFIG.load().wasted_cookie.iter().cloned());
        let dispatched = HashMap::new();

        // 创建事件通道
        let (event_tx, event_rx) = mpsc::channel(100);

        // 创建优先级队列
        let event_queue = Arc::new(Mutex::new(BinaryHeap::new()));

        // 创建通知器
        let event_notify = Arc::new(Notify::new());
        let sender = CookieEventSender { sender: event_tx };

        let manager = Self {
            valid,
            exhausted: exhaust,
            invalid,
            event_queue,
            event_notify,
            dispatched,
        };
        // 启动事件处理器
        spawn(manager.run(event_rx, sender.clone()));

        sender
    }

    /// Logs the current state of cookie collections
    /// Displays counts of valid, dispatched, exhausted, and invalid cookies
    fn log(&self) {
        info!(
            "Valid: {}, Dispatched: {}, Exhausted: {}, Invalid: {}",
            self.valid.len().to_string().green(),
            self.dispatched.len().to_string().blue(),
            self.exhausted.len().to_string().yellow(),
            self.invalid.len().to_string().red(),
        );
    }

    /// Saves the current state of cookies to the configuration
    /// Updates the cookie arrays in the config and writes to disk
    fn save(&mut self) {
        CLEWDR_CONFIG.rcu(|config| {
            let mut config = ClewdrConfig::clone(config);
            config.cookie_array = self
                .valid
                .iter()
                .chain(self.exhausted.iter())
                .cloned()
                .collect::<Vec<_>>();
            config.wasted_cookie = self.invalid.iter().cloned().collect();
            config
        });
        CLEWDR_CONFIG.load().save().unwrap_or_else(|e| {
            error!("Failed to save config: {}", e);
        });
    }

    /// Checks and resets cookies that have passed their reset time
    /// Moves reset cookies from exhausted to valid collection
    fn reset(&mut self) {
        let mut reset_cookies = Vec::new();
        self.exhausted.retain(|cookie| {
            let reset_cookie = cookie.clone().reset();
            if reset_cookie.reset_time.is_none() {
                reset_cookies.push(reset_cookie);
                false
            } else {
                true
            }
        });
        self.valid.extend(reset_cookies);
        self.save();
    }

    /// Dispatches a cookie for use
    /// Gets a cookie from the valid collection and moves it to dispatched
    ///
    /// # Returns
    /// * `Result<CookieStatus, ClewdrError>` - A cookie if available, error otherwise
    fn dispatch(&mut self) -> Result<CookieStatus, ClewdrError> {
        self.reset();
        let cookie = self
            .valid
            .pop_front()
            .ok_or(ClewdrError::NoCookieAvailable)?;
        let instant = Instant::now();
        self.dispatched.insert(cookie.clone(), instant);
        Ok(cookie)
    }

    /// Collects a returned cookie and processes it based on the return reason
    ///
    /// # Arguments
    /// * `cookie` - The cookie being returned
    /// * `reason` - Optional reason for the return that determines how the cookie is processed
    fn collect(&mut self, mut cookie: CookieStatus, reason: Option<Reason>) {
        let Some(_) = self.dispatched.remove(&cookie) else {
            return;
        };
        let Some(reason) = reason else {
            self.valid.push_back(cookie);
            return;
        };
        match reason {
            Reason::NormalPro => {
                self.valid.push_back(cookie);
            }
            Reason::TooManyRequest(i) => {
                cookie.reset_time = Some(i);
                self.exhausted.insert(cookie);
            }
            Reason::Restricted(i) => {
                cookie.reset_time = Some(i);
                self.exhausted.insert(cookie);
            }
            Reason::NonPro => {
                warn!("疑似爆米了, cookie: {}", cookie.cookie.to_string().red());
                self.invalid
                    .insert(UselessCookie::new(cookie.cookie, reason));
            }
            r => {
                self.invalid.insert(UselessCookie::new(cookie.cookie, r));
            }
        }
        self.save();
    }

    /// Accepts a new cookie into the valid collection
    /// Checks for duplicates before adding
    ///
    /// # Arguments
    /// * `cookie` - The new cookie to accept
    fn accept(&mut self, cookie: CookieStatus) {
        if CLEWDR_CONFIG.load().cookie_array.contains(&cookie)
            || CLEWDR_CONFIG
                .load()
                .wasted_cookie
                .iter()
                .any(|c| c.cookie == cookie.cookie)
        {
            warn!("Cookie already exists");
            return;
        }
        self.valid.push_back(cookie.clone());
        self.save();
    }

    /// Creates a report of all cookie statuses
    ///
    /// # Returns
    /// * `CookieStatusInfo` - Information about all cookie collections
    fn report(&self) -> CookieStatusInfo {
        CookieStatusInfo {
            valid: self.valid.iter().cloned().collect(),
            dispatched: self
                .dispatched
                .iter()
                .map(|(cookie, instant)| (cookie.clone(), instant.elapsed().as_secs()))
                .collect(),
            exhausted: self.exhausted.iter().cloned().collect(),
            invalid: self.invalid.iter().cloned().collect(),
        }
    }

    /// Checks for timed out cookies in the dispatched collection
    /// Moves expired cookies back to the valid collection
    fn check_timeout(&mut self) {
        // Handle timed out cookies
        let now = Instant::now();
        let expired: Vec<CookieStatus> = self
            .dispatched
            .iter()
            .filter(|(_, time)| now.duration_since(**time).as_secs() > 5 * 60)
            .map(|(cookie, _)| cookie.clone())
            .collect();

        for cookie in expired {
            warn!("Timing out dispatched cookie: {:?}", cookie);
            self.dispatched.remove(&cookie);
            self.valid.push_back(cookie);
        }
        self.reset();
    }

    /// Deletes a cookie from all collections
    ///
    /// # Arguments
    /// * `cookie` - The cookie to delete
    ///
    /// # Returns
    /// * `Result<(), ClewdrError>` - Success if found and deleted, error otherwise
    fn delete(&mut self, cookie: CookieStatus) -> Result<(), ClewdrError> {
        let mut found = false;
        self.valid.retain(|c| {
            if *c == cookie {
                found = true;
                false // remove
            } else {
                true // keep
            }
        });

        if self.dispatched.remove(&cookie).is_some() {
            found = true;
        }

        if self.exhausted.remove(&cookie) {
            found = true;
        }

        let cookie_info = &cookie.cookie;
        self.invalid.retain(|c| {
            if c.cookie == *cookie_info {
                found = true;
                false // remove
            } else {
                true // keep
            }
        });

        // Update config to reflect changes
        self.save();

        if found {
            Ok(())
        } else {
            Err(ClewdrError::UnexpectedNone)
        }
    }

    /// Spawns a task to listen for timer events and send timeout check events
    ///
    /// # Arguments
    /// * `interval` - The time interval for periodic checks
    /// * `event_tx` - Event sender to send timeout check events
    fn spawn_timeout_checker(mut interval: Interval, event_tx: CookieEventSender) {
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                if event_tx.check_timeout().await.is_err() {
                    break;
                }
            }
        });
    }

    /// Spawns a task to receive events and add them to the priority queue
    ///
    /// # Arguments
    /// * `event_rx` - Event receiver to get incoming events
    fn spawn_event_enqueuer(&self, mut event_rx: mpsc::Receiver<CookieEvent>) {
        let event_queue = self.event_queue.clone();
        let event_notify = self.event_notify.clone();

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                // 将事件添加到优先级队列
                {
                    event_queue.lock().await.push(event);
                }
                // 通知主循环有新事件
                event_notify.notify_one();
            }
        });
    }

    /// Main event processing loop
    /// Starts event receivers and processes events based on priority
    ///
    /// # Arguments
    /// * `event_rx` - Event receiver for incoming events
    /// * `event_sender` - Event sender for timeout checking
    async fn run(mut self, event_rx: mpsc::Receiver<CookieEvent>, event_sender: CookieEventSender) {
        // 启动事件接收器
        self.spawn_event_enqueuer(event_rx);
        // 启动超时检查协程
        let interval = tokio::time::interval(tokio::time::Duration::from_secs(INTERVAL));
        Self::spawn_timeout_checker(interval, event_sender);

        // 事件处理主循环
        loop {
            // 尝试从队列中获取事件
            let event = {
                let mut event_queue = self.event_queue.lock().await;
                event_queue.pop()
            };

            match event {
                Some(event) => {
                    // 处理事件
                    match event {
                        CookieEvent::Return(cookie, reason) => {
                            // 处理返回的cookie (最高优先级)
                            self.collect(cookie, reason);
                        }
                        CookieEvent::Submit(cookie) => {
                            // 处理提交的新cookie (次高优先级)
                            self.accept(cookie);
                        }
                        CookieEvent::CheckTimeout => {
                            // 处理超时检查 (中等优先级)
                            self.check_timeout();
                        }
                        CookieEvent::Request(sender) => {
                            // 处理请求 (最低优先级)
                            let cookie = self.dispatch();
                            if let Err(Ok(c)) = sender.send(cookie) {
                                error!("Failed to send cookie");
                                self.valid.push_back(c);
                            }
                        }
                        CookieEvent::GetStatus(sender) => {
                            let status_info = self.report();
                            sender.send(status_info).unwrap_or_else(|_| {
                                error!("Failed to send status info");
                            });
                        }
                        CookieEvent::Delete(cookie, sender) => {
                            let result = self.delete(cookie);
                            if sender.send(result).is_err() {
                                error!("Failed to send delete result");
                            }
                        }
                    }
                    self.log();
                }
                None => {
                    // 如果队列为空，等待通知
                    self.event_notify.notified().await;
                }
            }
        }
    }
}
