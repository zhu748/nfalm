use colored::Colorize;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Instant, Interval},
};
use tracing::{error, info, warn};

use crate::{
    config::{Config, CookieStatus, Reason, UselessCookie},
    error::ClewdrError,
};

// 定义统一的事件枚举，内置优先级顺序
#[derive(Debug)]
pub enum CookieEvent {
    // 返回Cookie (最高优先级)
    Return(CookieStatus, Option<Reason>),
    // 提交新的Cookie (次高优先级)
    Submit(CookieStatus),
    // 检查超时的Cookie (中等优先级)
    CheckTimeout,
    // 请求获取Cookie (最低优先级)
    Request(oneshot::Sender<Result<CookieStatus, ClewdrError>>),
}

// 为CookieEvent实现比较特性，用于优先级排序
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
        // 注意：我们返回 Reverse 排序，这样数字越小的优先级越高
        other.priority_value().cmp(&self.priority_value())
    }
}

impl CookieEvent {
    // 获取事件的优先级值
    fn priority_value(&self) -> u8 {
        match self {
            CookieEvent::Return(_, _) => 0, // 最高优先级
            CookieEvent::Submit(_) => 1,
            CookieEvent::CheckTimeout => 2,
            CookieEvent::Request(_) => 3, // 最低优先级
        }
    }
}

// Cookie管理器
pub struct CookieManager {
    valid: VecDeque<CookieStatus>,
    dispatched: HashMap<CookieStatus, Instant>,
    exhausted: HashSet<CookieStatus>,
    invalid: HashSet<UselessCookie>,
    event_sender: CookieEventSender,
    event_rx: Option<mpsc::Receiver<CookieEvent>>,
    event_queue: Arc<Mutex<BinaryHeap<CookieEvent>>>,
    event_notify: Arc<Notify>, // 添加一个通知器
    config: Config,
    interval: u64,
}

// 提供给外部的发送者接口
#[derive(Clone)]
pub struct CookieEventSender {
    sender: mpsc::Sender<CookieEvent>,
}

impl CookieEventSender {
    // 请求获取Cookie
    pub async fn request(&self) -> Result<CookieStatus, ClewdrError> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CookieEvent::Request(tx)).await?;
        rx.await?
    }

    // 返回Cookie
    pub async fn return_cookie(
        &self,
        cookie: CookieStatus,
        reason: Option<Reason>,
    ) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::Return(cookie, reason)).await
    }

    // 提交新Cookie
    pub async fn submit(
        &self,
        cookie: CookieStatus,
    ) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::Submit(cookie)).await
    }

    // 用于内部超时检查
    pub(crate) async fn check_timeout(&self) -> Result<(), mpsc::error::SendError<CookieEvent>> {
        self.sender.send(CookieEvent::CheckTimeout).await
    }
}

impl CookieManager {
    pub fn new(config: Config) -> (Self, CookieEventSender) {
        let mut config = config;
        config.cookie_array = config.cookie_array.into_iter().map(|c| c.reset()).collect();
        let valid = VecDeque::from_iter(
            config
                .cookie_array
                .iter()
                .filter(|c| c.reset_time.is_none())
                .cloned(),
        );
        let exhaust = HashSet::from_iter(
            config
                .cookie_array
                .iter()
                .filter(|c| c.reset_time.is_some())
                .cloned(),
        );
        let invalid = HashSet::from_iter(config.wasted_cookie.iter().cloned());
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
            event_sender: sender.clone(),
            event_rx: Some(event_rx),
            event_queue,
            event_notify,
            config,
            dispatched,
            interval: 300,
        };

        (manager, sender)
    }

    // 其他方法保持不变...
    fn log(&self) {
        info!(
            "Valid: {}, Dispatched: {}, Exhausted: {}, Invalid: {}",
            self.valid.len().to_string().green(),
            self.dispatched.len().to_string().blue(),
            self.exhausted.len().to_string().yellow(),
            self.invalid.len().to_string().red(),
        );
    }

    fn save(&mut self) {
        self.config.cookie_array = self
            .valid
            .iter()
            .chain(self.exhausted.iter())
            .chain(self.dispatched.keys())
            .cloned()
            .collect::<Vec<_>>();
        self.config.wasted_cookie = self.invalid.iter().cloned().collect();
        self.config.save().unwrap_or_else(|e| {
            error!("Failed to save config: {}", e);
        });
    }

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

    fn collect(&mut self, mut cookie: CookieStatus, reason: Option<Reason>) {
        let Some(_) = self.dispatched.remove(&cookie) else {
            return;
        };
        let Some(reason) = reason else {
            self.valid.push_back(cookie);
            return;
        };
        match reason {
            Reason::TooManyRequest(i) => {
                cookie.reset_time = Some(i);
                self.exhausted.insert(cookie);
            }
            Reason::Restricted(i) => {
                cookie.reset_time = Some(i);
                self.exhausted.insert(cookie);
            }
            Reason::NonPro => {
                warn!(
                    "疑似爆米了, id: {}, cookie: {}",
                    cookie.discord.unwrap_or_default().to_string().yellow(),
                    cookie.cookie.to_string().red()
                );
                self.invalid
                    .insert(UselessCookie::new(cookie.cookie, reason));
            }
            r => {
                self.invalid.insert(UselessCookie::new(cookie.cookie, r));
            }
        }
        self.save();
    }

    fn accept(&mut self, cookie: CookieStatus) {
        if self.config.cookie_array.contains(&cookie)
            || self
                .config
                .wasted_cookie
                .iter()
                .any(|c| c.cookie == cookie.cookie)
        {
            warn!("Cookie already exists");
            return;
        }
        self.config.cookie_array.push(cookie.clone());
        self.save();
        self.valid.push_back(cookie.clone());
    }

    fn check_timeout(&mut self) {
        // 处理超时的cookie
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

    // 启动协程监听定时器并发送超时检查事件
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

    fn spawn_event_enqueuer(&mut self) {
        let event_queue = self.event_queue.clone();
        let event_notify = self.event_notify.clone();
        let mut event_rx = self.event_rx.take().expect("Should not be None");

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

    pub async fn run(mut self) {
        // 启动事件接收器
        self.spawn_event_enqueuer();

        // 启动超时检查协程
        let interval = tokio::time::interval(tokio::time::Duration::from_secs(self.interval));
        Self::spawn_timeout_checker(interval, self.event_sender.clone());

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
                            if let Err(e) = sender.send(cookie) {
                                error!("Failed to send cookie");
                                if let Ok(c) = e {
                                    self.valid.push_back(c);
                                }
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
