//! Runtime Abstraction Layer
//!
//! Provides a platform-agnostic async runtime interface that works on both
//! native platforms (using Tokio) and WebAssembly (using browser APIs).
//!
//! This module follows the Hexagonal Architecture pattern, isolating platform
//! specifics behind a trait boundary.

// =============================================================================
// Runtime Trait Definition
// =============================================================================

use std::future::Future;
use std::pin::Pin;

use std::task::{Context, Poll};
use std::time::Duration;

/// Platform-agnostic async runtime trait.
///
/// Implementations:
/// - `TokioRuntime`: Native platforms (Linux, macOS, Windows)
/// - `WasmRuntime`: WebAssembly (browsers, edge compute)
///
/// # Send/Sync Requirements
///
/// All futures must be `Send` to satisfy Tokio's multi-threaded executor.
/// This is stricter than WASM needs (single-threaded), but ensures portability.
pub trait Runtime: Send + Sync + Clone + 'static {
    /// Spawn a new asynchronous task.
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future<Output: Send> + Send + 'static;

    /// Sleep for the specified duration.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;

    /// Create an interval stream that ticks every `period`.
    fn interval(&self, period: Duration) -> Interval;

    /// Create a bounded channel.
    fn channel<T>(&self, capacity: usize) -> (Sender<T>, Receiver<T>)
    where
        T: Send + 'static;

    /// Get current time.
    fn now(&self) -> Instant;

    /// Wrap a future with a timeout.
    fn timeout<F>(&self, duration: Duration, future: F) -> Timeout<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send;
}

// =============================================================================
// Native Implementation (Tokio)
// =============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::*;
    use tokio::sync::mpsc;
    use tokio::time::{Instant as TokioInstant, Interval as TokioInterval};

    #[derive(Clone, Debug, Default)]
    pub struct TokioRuntime;

    impl TokioRuntime {
        pub fn new() -> Self {
            Self
        }
    }

    impl super::Runtime for TokioRuntime {
        fn spawn<F>(&self, future: F) -> super::JoinHandle<F::Output>
        where
            F: Future<Output: Send> + Send + 'static,
        {
            super::JoinHandle {
                inner: super::JoinHandleInner::Tokio(tokio::spawn(future)),
            }
        }

        fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
            tokio::time::sleep(duration)
        }

        fn interval(&self, period: Duration) -> super::Interval {
            super::Interval {
                inner: super::IntervalInner::Tokio(tokio::time::interval(period)),
            }
        }

        fn channel<T>(&self, capacity: usize) -> (super::Sender<T>, super::Receiver<T>)
        where
            T: Send + 'static,
        {
            let (tx, rx) = mpsc::channel(capacity);
            (
                super::Sender {
                    inner: super::SenderInner::Tokio(tx),
                },
                super::Receiver {
                    inner: super::ReceiverInner::Tokio(rx),
                },
            )
        }

        fn now(&self) -> super::Instant {
            super::Instant {
                inner: super::InstantInner::Tokio(TokioInstant::now()),
            }
        }

        fn timeout<F>(&self, duration: Duration, future: F) -> super::Timeout<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send,
        {
            let timeout: Pin<Box<dyn Future<Output = Result<F::Output, tokio::time::error::Elapsed>> + Send>> = 
                Box::pin(tokio::time::timeout(duration, future));
            super::Timeout {
                inner: super::TimeoutInner::Tokio { timeout },
            }
        }
    }

    #[allow(dead_code)]
    pub type NativeJoinHandle<T> = tokio::task::JoinHandle<T>;
    #[allow(dead_code)]
    pub type NativeInterval = TokioInterval;
    #[allow(dead_code)]
    pub type NativeSender<T> = mpsc::Sender<T>;
    #[allow(dead_code)]
    pub type NativeReceiver<T> = mpsc::Receiver<T>;
    #[allow(dead_code)]
    pub type NativeInstant = TokioInstant;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_impl::TokioRuntime;

// =============================================================================
// WASM Implementation
// =============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::*;
    use futures::channel::mpsc;
    use wasm_bindgen::prelude::*;

    #[derive(Clone, Debug, Default)]
    pub struct WasmRuntime;

    impl WasmRuntime {
        pub fn new() -> Self {
            Self
        }
    }

    impl super::Runtime for WasmRuntime {
        fn spawn<F>(&self, future: F) -> super::JoinHandle<F::Output>
        where
            F: Future<Output: Send> + Send + 'static,
        {
            let (remote, handle) = future.remote_handle();
            wasm_bindgen_futures::spawn_local(remote);
            super::JoinHandle {
                inner: super::JoinHandleInner::Wasm { handle },
            }
        }

        fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
            WasmSleep::new(duration)
        }

        fn interval(&self, period: Duration) -> super::Interval {
            super::Interval {
                inner: super::IntervalInner::Wasm(WasmInterval::new(period)),
            }
        }

        fn channel<T>(&self, capacity: usize) -> (super::Sender<T>, super::Receiver<T>)
        where
            T: Send + 'static,
        {
            let (tx, rx) = mpsc::channel(capacity);
            (
                super::Sender {
                    inner: super::SenderInner::Wasm(tx),
                },
                super::Receiver {
                    inner: super::ReceiverInner::Wasm(rx),
                },
            )
        }

        fn now(&self) -> super::Instant {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            super::Instant {
                inner: super::InstantInner::Wasm(performance.now()),
            }
        }

        fn timeout<F>(&self, duration: Duration, future: F) -> super::Timeout<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send,
        {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            let now = performance.now();
            let deadline = now + duration.as_millis() as f64;

            super::Timeout {
                inner: super::TimeoutInner::Wasm {
                    deadline,
                    future: Box::pin(future),
                },
            }
        }
    }

    pub struct WasmSleep {
        target_time: f64,
    }

    impl WasmSleep {
        fn new(duration: Duration) -> Self {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            let now = performance.now();
            Self {
                target_time: now + duration.as_millis() as f64,
            }
        }
    }

    impl Future for WasmSleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            let now = performance.now();

            if now >= self.target_time {
                Poll::Ready(())
            } else {
                let waker = cx.waker().clone();
                let remaining = (self.target_time - now) as i32;

                let closure = Closure::once_into_js(move || {
                    waker.wake();
                });

                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        remaining.max(0),
                    )
                    .expect("set_timeout failed");

                Poll::Pending
            }
        }
    }

    pub struct WasmInterval {
        period_ms: f64,
        next_tick: f64,
        is_first: bool,
    }

    impl WasmInterval {
        fn new(period: Duration) -> Self {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            let now = performance.now();

            Self {
                period_ms: period.as_millis() as f64,
                next_tick: now,
                is_first: true,
            }
        }
    }

    impl Future for WasmInterval {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let window = web_sys::window().expect("no window");
            let performance = window.performance().expect("no performance");
            let now = performance.now();

            if self.is_first {
                self.is_first = false;
                self.next_tick = now + self.period_ms;
                return Poll::Ready(());
            }

            if now >= self.next_tick {
                self.next_tick = now + self.period_ms;
                Poll::Ready(())
            } else {
                let waker = cx.waker().clone();
                let remaining = (self.next_tick - now) as i32;

                let closure = Closure::once_into_js(move || {
                    waker.wake();
                });

                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        remaining.max(0),
                    )
                    .expect("set_timeout failed");

                Poll::Pending
            }
        }
    }

    #[allow(dead_code)]
    pub type WasmJoinHandle<T> = futures::future::RemoteHandle<T>;
    #[allow(dead_code)]
    pub type WasmSender<T> = mpsc::Sender<T>;
    #[allow(dead_code)]
    pub type WasmReceiver<T> = mpsc::Receiver<T>;
    #[allow(dead_code)]
    pub type WasmInstant = f64;
}

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::WasmRuntime;

// =============================================================================
// Supporting Types (Platform-Agnostic Wrappers)
// =============================================================================

/// Handle to a spawned task.
pub struct JoinHandle<T> {
    inner: JoinHandleInner<T>,
}

enum JoinHandleInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::task::JoinHandle<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm { handle: futures::future::RemoteHandle<T> },
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().inner {
            #[cfg(not(target_arch = "wasm32"))]
            JoinHandleInner::Tokio(handle) => Pin::new(handle).poll(cx).map(|r| r.expect("task panicked")),
            #[cfg(target_arch = "wasm32")]
            JoinHandleInner::Wasm { handle } => Pin::new(handle).poll(cx),
        }
    }
}

/// Interval for periodic tasks.
pub struct Interval {
    inner: IntervalInner,
}

enum IntervalInner {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::time::Interval),
    #[cfg(target_arch = "wasm32")]
    Wasm(wasm_impl::WasmInterval),
}

impl Interval {
    /// Wait for the next tick.
    pub async fn tick(&mut self) -> Instant {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            IntervalInner::Tokio(interval) => {
                let _ = interval.tick().await;
                Instant {
                    inner: InstantInner::Tokio(tokio::time::Instant::now()),
                }
            }
            #[cfg(target_arch = "wasm32")]
            IntervalInner::Wasm(interval) => {
                use std::future::Future;
                Pin::new(interval).await;
                let window = web_sys::window().expect("no window");
                let performance = window.performance().expect("no performance");
                Instant {
                    inner: InstantInner::Wasm(performance.now()),
                }
            }
        }
    }
}

impl Future for Interval {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match &mut this.inner {
            #[cfg(not(target_arch = "wasm32"))]
            IntervalInner::Tokio(interval) => Pin::new(interval).poll_tick(cx).map(|_| ()),
            #[cfg(target_arch = "wasm32")]
            IntervalInner::Wasm(interval) => Pin::new(interval).poll(cx),
        }
    }
}

/// Channel sender.
pub struct Sender<T> {
    inner: SenderInner<T>,
}

enum SenderInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::sync::mpsc::Sender<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm(futures::channel::mpsc::Sender<T>),
}

impl<T> Sender<T> {
    /// Send a value through the channel.
    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            SenderInner::Tokio(sender) => sender.send(value).await.map_err(|e| SendError(e.0)),
            #[cfg(target_arch = "wasm32")]
            SenderInner::Wasm(sender) => {
                let mut sender = sender.clone();
                sender.send(value).await.map_err(|e| SendError(e.0))
            }
        }
    }
}

impl<T: Clone> Clone for Sender<T> {
    fn clone(&self) -> Self {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            SenderInner::Tokio(sender) => Self {
                inner: SenderInner::Tokio(sender.clone()),
            },
            #[cfg(target_arch = "wasm32")]
            SenderInner::Wasm(sender) => Self {
                inner: SenderInner::Wasm(sender.clone()),
            },
        }
    }
}

/// Channel receiver.
pub struct Receiver<T> {
    inner: ReceiverInner<T>,
}

enum ReceiverInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::sync::mpsc::Receiver<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm(futures::channel::mpsc::Receiver<T>),
}

impl<T> Receiver<T> {
    /// Receive the next value.
    pub async fn recv(&mut self) -> Option<T> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            ReceiverInner::Tokio(receiver) => receiver.recv().await,
            #[cfg(target_arch = "wasm32")]
            ReceiverInner::Wasm(receiver) => {
                use futures::stream::StreamExt;
                receiver.next().await
            }
        }
    }
}

/// Error when sending to a closed channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendError<T>(pub T);

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "send failed: channel closed")
    }
}

impl<T: std::fmt::Debug> std::error::Error for SendError<T> {}

/// Platform-agnostic instant in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instant {
    inner: InstantInner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstantInner {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::time::Instant),
    #[cfg(target_arch = "wasm32")]
    Wasm(f64),
}

impl Instant {
    /// Duration since another instant.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        match (&self.inner, &earlier.inner) {
            #[cfg(not(target_arch = "wasm32"))]
            (InstantInner::Tokio(s), InstantInner::Tokio(e)) => s.duration_since(*e),
            #[cfg(target_arch = "wasm32")]
            (InstantInner::Wasm(s), InstantInner::Wasm(e)) => {
                Duration::from_millis((s - e).max(0.0) as u64)
            }
        }
    }

    /// Elapsed time since this instant.
    pub fn elapsed(&self) -> Duration {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            InstantInner::Tokio(i) => i.elapsed(),
            #[cfg(target_arch = "wasm32")]
            InstantInner::Wasm(t) => {
                let window = web_sys::window().expect("no window");
                let performance = window.performance().expect("no performance");
                let now = performance.now();
                Duration::from_millis((now - t).max(0.0) as u64)
            }
        }
    }
}

/// Timeout wrapper future.
pub struct Timeout<T> {
    inner: TimeoutInner<T>,
}

enum TimeoutInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio {
        timeout: Pin<Box<dyn Future<Output = Result<T, tokio::time::error::Elapsed>> + Send>>,
    },
    #[cfg(target_arch = "wasm32")]
    Wasm {
        deadline: f64,
        future: Pin<Box<dyn Future<Output = T> + Send>>,
    },
}

/// Error returned when a timeout expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timeout expired")
    }
}

impl std::error::Error for TimeoutError {}

impl<T> Future for Timeout<T> {
    type Output = Result<T, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        match &mut this.inner {
            #[cfg(not(target_arch = "wasm32"))]
            TimeoutInner::Tokio { timeout } => {
                match Future::poll(timeout.as_mut(), cx) {
                    Poll::Ready(Ok(v)) => Poll::Ready(Ok(v)),
                    Poll::Ready(Err(_)) => Poll::Ready(Err(TimeoutError)),
                    Poll::Pending => Poll::Pending,
                }
            }
            #[cfg(target_arch = "wasm32")]
            TimeoutInner::Wasm { deadline, future } => {
                let window = web_sys::window().expect("no window");
                let performance = window.performance().expect("no performance");
                let now = performance.now();

                if now >= *deadline {
                    return Poll::Ready(Err(TimeoutError));
                }

                match Future::poll(future.as_mut(), cx) {
                    Poll::Ready(v) => Poll::Ready(Ok(v)),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

// =============================================================================
// Default Runtime Type Aliases
// =============================================================================

/// Default runtime for the current platform.
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultRuntime = TokioRuntime;

#[cfg(target_arch = "wasm32")]
pub type DefaultRuntime = WasmRuntime;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runtime_type() {
        fn _check_runtime<R: Runtime>() {}
        _check_runtime::<DefaultRuntime>();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tokio_runtime_spawn() {
        let runtime = TokioRuntime::new();
        let handle = runtime.spawn(async { 42 });
        let result = handle.await;
        assert_eq!(result, 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tokio_runtime_channel() {
        let runtime = TokioRuntime::new();
        let (tx, mut rx) = runtime.channel::<i32>(10);

        tx.send(42).await.unwrap();
        let received = rx.recv().await;
        assert_eq!(received, Some(42));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tokio_runtime_sleep() {
        let runtime = TokioRuntime::new();
        let start = runtime.now();

        runtime.sleep(Duration::from_millis(10)).await;

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tokio_runtime_timeout_success() {
        let runtime = TokioRuntime::new();
        let fut: std::pin::Pin<Box<dyn Future<Output = i32> + Send>> = Box::pin(async { 42 });
        let result = runtime.timeout(Duration::from_secs(1), fut).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tokio_runtime_timeout_expires() {
        let runtime = TokioRuntime::new();
        let fut: std::pin::Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        let result = runtime.timeout(Duration::from_millis(10), fut).await;
        assert!(result.is_err());
    }
}
