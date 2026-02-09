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

pub mod sync;

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
    /// Create a new instance of the runtime.
    fn new() -> Self;
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

    /// Yield execution back to the runtime.
    ///
    /// This allows other tasks to run, preventing blocking the async runtime
    /// during CPU-intensive operations.
    fn yield_now(&self) -> impl Future<Output = ()> + Send;

    /// Create a watch channel for state broadcasting.
    ///
    /// Watch channels are similar to broadcast channels but always keep
    /// the most recent value. New receivers immediately get the current value.
    fn watch_channel<T>(&self, initial: T) -> (WatchSender<T>, WatchReceiver<T>)
    where
        T: Clone + Send + Sync + 'static;
}

/// Helper trait for runtime construction that doesn't require Send.
///
/// This allows WASM runtimes to be constructed in non-Send contexts.
pub trait RuntimeExt: Runtime {
    /// Create a new runtime (may not be Send on WASM).
    fn create() -> Self;
}

impl<R: Runtime> RuntimeExt for R {
    fn create() -> Self {
        R::new()
    }
}

// =============================================================================
// Default Runtime Type Alias
// =============================================================================

/// The default runtime for the current platform.
///
/// - On native: Uses `TokioRuntime`
/// - On WASM: Uses `WasmRuntime`
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultRuntime = TokioRuntime;

#[cfg(target_arch = "wasm32")]
pub type DefaultRuntime = WasmRuntime;

// =============================================================================
// Native Implementation (Tokio)
// =============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::*;
    use tokio::sync::{mpsc, watch};
    use tokio::time::{Instant as TokioInstant, Interval as TokioInterval};

    #[derive(Clone, Debug, Default)]
    pub struct TokioRuntime;

    impl TokioRuntime {
        pub fn new() -> Self {
            Self
        }
    }

    impl super::Runtime for TokioRuntime {
        fn new() -> Self {
            Self
        }

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
            let timeout: Pin<
                Box<dyn Future<Output = Result<F::Output, tokio::time::error::Elapsed>> + Send>,
            > = Box::pin(tokio::time::timeout(duration, future));
            super::Timeout {
                inner: super::TimeoutInner::Tokio { timeout },
            }
        }

        fn yield_now(&self) -> impl Future<Output = ()> + Send {
            tokio::task::yield_now()
        }

        fn watch_channel<T>(&self, initial: T) -> (super::WatchSender<T>, super::WatchReceiver<T>)
        where
            T: Clone + Send + Sync + 'static,
        {
            let (tx, rx) = watch::channel(initial);
            (
                super::WatchSender {
                    inner: super::WatchSenderInner::Tokio(tx),
                },
                super::WatchReceiver {
                    inner: super::WatchReceiverInner::Tokio(rx),
                },
            )
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
    use futures::channel::{mpsc, oneshot};
    use std::sync::{Arc, Mutex};
    use wasm_bindgen::prelude::*;

    #[derive(Clone, Debug, Default)]
    pub struct WasmRuntime;

    impl WasmRuntime {
        pub fn new() -> Self {
            Self
        }
    }

    impl super::Runtime for WasmRuntime {
        fn new() -> Self {
            Self
        }

        fn spawn<F>(&self, future: F) -> super::JoinHandle<F::Output>
        where
            F: Future<Output: Send> + Send + 'static,
        {
            let (tx, rx) = oneshot::channel();
            wasm_bindgen_futures::spawn_local(async move {
                let result = future.await;
                let _ = tx.send(result);
            });
            super::JoinHandle {
                inner: super::JoinHandleInner::Wasm { receiver: rx },
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

        fn yield_now(&self) -> impl Future<Output = ()> + Send {
            // WASM is single-threaded, so yield is essentially a no-op
            std::future::ready(())
        }

        fn watch_channel<T>(&self, initial: T) -> (super::WatchSender<T>, super::WatchReceiver<T>)
        where
            T: Clone + Send + Sync + 'static,
        {
            let state = Arc::new(Mutex::new(initial));
            let (tx, rx) = mpsc::channel(1);

            (
                super::WatchSender {
                    inner: super::WatchSenderInner::Wasm {
                        state: Arc::clone(&state),
                        _notify: tx,
                    },
                },
                super::WatchReceiver {
                    inner: super::WatchReceiverInner::Wasm {
                        state,
                        receiver: rx,
                        current: None,
                    },
                },
            )
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

    /// Simple watch channel implementation for WASM
    #[allow(dead_code)]
    pub struct WasmWatch<T> {
        value: Arc<Mutex<T>>,
        version: Arc<Mutex<u64>>,
    }

    #[allow(dead_code)]
    impl<T: Clone> WasmWatch<T> {
        fn new(initial: T) -> Self {
            Self {
                value: Arc::new(Mutex::new(initial)),
                version: Arc::new(Mutex::new(0)),
            }
        }

        fn send(&self, value: T) {
            let mut v = self.value.lock().unwrap();
            *v = value;
            let mut ver = self.version.lock().unwrap();
            *ver += 1;
        }

        fn borrow(&self) -> std::sync::MutexGuard<'_, T> {
            self.value.lock().unwrap()
        }

        fn version(&self) -> u64 {
            *self.version.lock().unwrap()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::WasmRuntime;

// =============================================================================
// Supporting Types
// =============================================================================

/// A handle to a spawned task.
pub struct JoinHandle<T> {
    inner: JoinHandleInner<T>,
}

enum JoinHandleInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::task::JoinHandle<T>),
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)]
    Dummy,
    #[cfg(target_arch = "wasm32")]
    Wasm {
        receiver: futures::channel::oneshot::Receiver<T>,
    },
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().inner {
            #[cfg(not(target_arch = "wasm32"))]
            JoinHandleInner::Tokio(handle) => Pin::new(handle).poll(cx).map(|r| r.unwrap()),
            #[cfg(not(target_arch = "wasm32"))]
            JoinHandleInner::Dummy => Poll::Pending,
            #[cfg(target_arch = "wasm32")]
            JoinHandleInner::Wasm { receiver } => Pin::new(receiver).poll(cx).map(|r| r.unwrap()),
        }
    }
}

/// An interval that ticks at a regular period.
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
    pub async fn tick(&mut self) {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            IntervalInner::Tokio(interval) => {
                interval.tick().await;
            }
            #[cfg(target_arch = "wasm32")]
            IntervalInner::Wasm(interval) => {
                std::future::poll_fn(|cx| Pin::new(&mut *interval).poll(cx)).await;
            }
        }
    }
}

impl Future for Interval {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().inner {
            #[cfg(not(target_arch = "wasm32"))]
            IntervalInner::Tokio(interval) => Pin::new(interval).poll_tick(cx).map(|_| ()),
            #[cfg(target_arch = "wasm32")]
            IntervalInner::Wasm(interval) => Pin::new(interval).poll(cx),
        }
    }
}

/// A sender for a bounded channel.
pub struct Sender<T> {
    inner: SenderInner<T>,
}

enum SenderInner<T> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::sync::mpsc::Sender<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm(futures::channel::mpsc::Sender<T>),
}

impl<T: Clone> Sender<T> {
    /// Send a value on the channel.
    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            SenderInner::Tokio(tx) => tx.send(value).await.map_err(|e| SendError(e.0)),
            #[cfg(target_arch = "wasm32")]
            SenderInner::Wasm(tx) => {
                use futures::SinkExt;
                let value_clone = value.clone();
                let mut tx = tx.clone();
                tx.send(value).await.map_err(|_| SendError(value_clone))
            }
        }
    }
}

/// A receiver for a bounded channel.
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
    /// Receive a value from the channel.
    pub async fn recv(&mut self) -> Option<T> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            ReceiverInner::Tokio(rx) => rx.recv().await,
            #[cfg(target_arch = "wasm32")]
            ReceiverInner::Wasm(rx) => {
                use futures::StreamExt;
                rx.next().await
            }
        }
    }
}

impl<T> futures::Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().inner {
            #[cfg(not(target_arch = "wasm32"))]
            ReceiverInner::Tokio(rx) => Pin::new(rx).poll_recv(cx),
            #[cfg(target_arch = "wasm32")]
            ReceiverInner::Wasm(rx) => Pin::new(rx).poll_next(cx),
        }
    }
}

/// Error when sending on a closed channel.
pub struct SendError<T>(pub T);

impl<T> std::fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendError").finish()
    }
}

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel closed")
    }
}

impl<T: Send> std::error::Error for SendError<T> {}

/// An instant in time.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Instant {
    inner: InstantInner,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum InstantInner {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::time::Instant),
    #[cfg(target_arch = "wasm32")]
    Wasm(f64), // Performance.now() in milliseconds
}

impl Instant {
    /// Get the duration since another instant.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        match (&self.inner, earlier.inner) {
            #[cfg(not(target_arch = "wasm32"))]
            (InstantInner::Tokio(now), InstantInner::Tokio(then)) => now.duration_since(then),
            #[cfg(target_arch = "wasm32")]
            (InstantInner::Wasm(now), InstantInner::Wasm(then)) => {
                Duration::from_millis((now - then) as u64)
            }
            #[cfg(all(not(target_arch = "wasm32"), target_arch = "wasm32"))]
            _ => unreachable!(),
            #[cfg(all(target_arch = "wasm32", not(target_arch = "wasm32")))]
            _ => unreachable!(),
        }
    }

    /// Get the elapsed time since this instant.
    /// Note: This requires a Runtime to get the current time.
    /// Use `runtime.now().duration_since(instant)` instead.
    pub fn elapsed_with_runtime<R: Runtime>(&self, runtime: &R) -> Duration {
        runtime.now().duration_since(self.clone())
    }
}

/// A future that times out after a duration.
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
        future: Pin<Box<dyn Future<Output = T>>>,
    },
}

impl<T> Future for Timeout<T> {
    type Output = Result<T, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().inner {
            #[cfg(not(target_arch = "wasm32"))]
            TimeoutInner::Tokio { timeout } => match timeout.as_mut().poll(cx) {
                Poll::Ready(Ok(v)) => Poll::Ready(Ok(v)),
                Poll::Ready(Err(_)) => Poll::Ready(Err(TimeoutError)),
                Poll::Pending => Poll::Pending,
            },
            #[cfg(target_arch = "wasm32")]
            TimeoutInner::Wasm { deadline, future } => {
                let window = web_sys::window().expect("no window");
                let performance = window.performance().expect("no performance");
                let now = performance.now();

                if now >= *deadline {
                    return Poll::Ready(Err(TimeoutError));
                }

                match future.as_mut().poll(cx) {
                    Poll::Ready(v) => Poll::Ready(Ok(v)),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

/// Error when a timeout expires.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timeout expired")
    }
}

impl std::error::Error for TimeoutError {}

// =============================================================================
// Watch Channel Types
// =============================================================================

/// A sender for a watch channel.
pub struct WatchSender<T: Clone> {
    inner: WatchSenderInner<T>,
}

impl<T: Clone> Clone for WatchSender<T> {
    fn clone(&self) -> Self {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchSenderInner::Tokio(tx) => Self {
                inner: WatchSenderInner::Tokio(tx.clone()),
            },
            #[cfg(target_arch = "wasm32")]
            WatchSenderInner::Wasm { state, _notify: _ } => {
                let (_tx, _rx) = futures::channel::mpsc::channel(1);
                Self {
                    inner: WatchSenderInner::Wasm {
                        state: std::sync::Arc::clone(state),
                        _notify: _tx,
                    },
                }
            }
        }
    }
}

#[derive(Clone)]
enum WatchSenderInner<T: Clone> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::sync::watch::Sender<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm {
        state: std::sync::Arc<std::sync::Mutex<T>>,
        _notify: futures::channel::mpsc::Sender<()>,
    },
}

impl<T: Clone + Send + Sync> WatchSender<T> {
    /// Send a value on the channel.
    pub fn send(&self, value: T) -> Result<(), WatchSendError<T>> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchSenderInner::Tokio(tx) => tx.send(value).map_err(|e| WatchSendError(e.0)),
            #[cfg(target_arch = "wasm32")]
            WatchSenderInner::Wasm { state, .. } => {
                let mut s = state.lock().unwrap();
                *s = value;
                Ok(())
            }
        }
    }
}

/// A receiver for a watch channel.
pub struct WatchReceiver<T: Clone> {
    inner: WatchReceiverInner<T>,
}

enum WatchReceiverInner<T: Clone> {
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::sync::watch::Receiver<T>),
    #[cfg(target_arch = "wasm32")]
    Wasm {
        state: std::sync::Arc<std::sync::Mutex<T>>,
        #[allow(dead_code)]
        receiver: futures::channel::mpsc::Receiver<()>,
        current: Option<T>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Clone> Clone for WatchReceiverInner<T> {
    fn clone(&self) -> Self {
        match self {
            WatchReceiverInner::Tokio(rx) => WatchReceiverInner::Tokio(rx.clone()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: Clone> Clone for WatchReceiverInner<T> {
    fn clone(&self) -> Self {
        match self {
            WatchReceiverInner::Wasm {
                state,
                receiver: _,
                current,
            } => {
                // Create a new channel for this receiver
                let (_tx, new_rx) = futures::channel::mpsc::channel(1);
                WatchReceiverInner::Wasm {
                    state: std::sync::Arc::clone(state),
                    receiver: new_rx,
                    current: current.clone(),
                }
            }
        }
    }
}

impl<T: Clone + Send + Sync> Clone for WatchReceiver<T> {
    fn clone(&self) -> Self {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchReceiverInner::Tokio(rx) => Self {
                inner: WatchReceiverInner::Tokio(rx.clone()),
            },
            #[cfg(target_arch = "wasm32")]
            WatchReceiverInner::Wasm {
                state,
                receiver: _,
                current,
            } => {
                // Create a new channel for this receiver
                let (_tx, new_rx) = futures::channel::mpsc::channel(1);
                Self {
                    inner: WatchReceiverInner::Wasm {
                        state: std::sync::Arc::clone(state),
                        receiver: new_rx,
                        current: current.clone(),
                    },
                }
            }
        }
    }
}

impl<T: Clone + Send + Sync + PartialEq> WatchReceiver<T> {
    /// Borrow the current value.
    pub fn borrow(&self) -> std::sync::MutexGuard<'_, T>
    where
        T: 'static,
    {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchReceiverInner::Tokio(_rx) => {
                // For Tokio, we need to return something with the right lifetime
                // This is a bit tricky - for now we'll use a simplified approach
                unimplemented!("Use borrow_and_update for Tokio")
            }
            #[cfg(target_arch = "wasm32")]
            WatchReceiverInner::Wasm { state, .. } => state.lock().unwrap(),
        }
    }

    /// Check if the channel has been closed.
    pub fn has_changed(&self) -> Result<bool, WatchRecvError> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchReceiverInner::Tokio(rx) => rx.has_changed().map_err(|_| WatchRecvError::Closed),
            #[cfg(target_arch = "wasm32")]
            WatchReceiverInner::Wasm { state, current, .. } => {
                let s = state.lock().unwrap();
                match current {
                    Some(c) if *c == *s => Ok(false),
                    _ => Ok(true), // Report changed for simplicity
                }
            }
        }
    }

    /// Get a changed notification.
    pub async fn changed(&mut self) -> Result<(), WatchRecvError> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchReceiverInner::Tokio(rx) => rx.changed().await.map_err(|_| WatchRecvError::Closed),
            #[cfg(target_arch = "wasm32")]
            WatchReceiverInner::Wasm { receiver, .. } => {
                use futures::StreamExt;
                receiver.next().await.ok_or(WatchRecvError::Closed)
            }
        }
    }

    /// Borrow and update the current value.
    pub fn borrow_and_update(&mut self) -> T {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            WatchReceiverInner::Tokio(rx) => rx.borrow_and_update().clone(),
            #[cfg(target_arch = "wasm32")]
            WatchReceiverInner::Wasm { state, current, .. } => {
                let s = state.lock().unwrap();
                let value = s.clone();
                *current = Some(value.clone());
                value
            }
        }
    }
}

/// Error when sending on a closed watch channel.
pub struct WatchSendError<T>(pub T);

impl<T> std::fmt::Debug for WatchSendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchSendError").finish()
    }
}

impl<T> std::fmt::Display for WatchSendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "watch channel closed")
    }
}

impl<T: Send> std::error::Error for WatchSendError<T> {}

/// Error when receiving from a watch channel.
#[derive(Debug, Clone, Copy)]
pub enum WatchRecvError {
    Closed,
}

impl std::fmt::Display for WatchRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchRecvError::Closed => write!(f, "watch channel closed"),
        }
    }
}

impl std::error::Error for WatchRecvError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_runtime_spawn() {
        let runtime = TokioRuntime::new();
        let handle = runtime.spawn(async { 42 });
        let result = handle.await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_runtime_sleep() {
        let runtime = TokioRuntime::new();
        let start = std::time::Instant::now();
        runtime.sleep(Duration::from_millis(10)).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_runtime_channel() {
        let runtime = TokioRuntime::new();
        let (tx, mut rx) = runtime.channel::<i32>(10);

        tx.send(42).await.unwrap();
        let value = rx.recv().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_runtime_interval() {
        let runtime = TokioRuntime::new();
        let mut interval = runtime.interval(Duration::from_millis(10));

        interval.tick().await;
        interval.tick().await;
        // If we get here, the interval is working
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_runtime_watch_channel() {
        let runtime = TokioRuntime::new();
        let (tx, mut rx) = runtime.watch_channel(false);

        tx.send(true).unwrap();
        rx.changed().await.unwrap();
        let value = rx.borrow_and_update();
        assert!(value);
    }
}
