//! Synchronization primitives for the Runtime abstraction.
//!
//! This module provides platform-agnostic synchronization primitives that work
//! on both native (Tokio) and WebAssembly platforms.

// =============================================================================
// RwLock - Platform-agnostic async RwLock
// =============================================================================

/// Platform-agnostic async read-write lock.
///
/// On native: Uses `tokio::sync::RwLock`
/// On WASM: Uses `std::sync::RwLock` (WASM is single-threaded)
pub struct RwLock<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::sync::RwLock<T>,
    #[cfg(target_arch = "wasm32")]
    inner: std::sync::RwLock<T>,
}

impl<T> RwLock<T> {
    /// Create a new RwLock.
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            inner: tokio::sync::RwLock::new(value),
            #[cfg(target_arch = "wasm32")]
            inner: std::sync::RwLock::new(value),
        }
    }

    /// Acquire a read lock.
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard {
            #[cfg(not(target_arch = "wasm32"))]
            guard: self.inner.read().await,
            #[cfg(target_arch = "wasm32")]
            guard: self.inner.read().unwrap(),
        }
    }

    /// Acquire a write lock.
    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        RwLockWriteGuard {
            #[cfg(not(target_arch = "wasm32"))]
            guard: self.inner.write().await,
            #[cfg(target_arch = "wasm32")]
            guard: self.inner.write().unwrap(),
        }
    }

    /// Acquire a read lock (blocking, for sync contexts).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn blocking_read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard {
            guard: self.inner.blocking_read(),
        }
    }

    /// Acquire a read lock (blocking, for sync contexts) - WASM fallback.
    #[cfg(target_arch = "wasm32")]
    pub fn blocking_read(&self) -> RwLockReadGuard<'_, T> {
        // WASM is single-threaded, so just acquire the lock
        RwLockReadGuard {
            guard: self.inner.read().unwrap(),
        }
    }

    /// Try to acquire a read lock.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.inner.try_read().ok().map(|guard| RwLockReadGuard { guard })
    }

    /// Try to acquire a read lock - WASM fallback.
    #[cfg(target_arch = "wasm32")]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.inner.try_read().ok().map(|guard| RwLockReadGuard { guard })
    }
}

/// Read guard for RwLock.
pub struct RwLockReadGuard<'a, T> {
    #[cfg(not(target_arch = "wasm32"))]
    guard: tokio::sync::RwLockReadGuard<'a, T>,
    #[cfg(target_arch = "wasm32")]
    guard: std::sync::RwLockReadGuard<'a, T>,
}

impl<T> std::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// Write guard for RwLock.
pub struct RwLockWriteGuard<'a, T> {
    #[cfg(not(target_arch = "wasm32"))]
    guard: tokio::sync::RwLockWriteGuard<'a, T>,
    #[cfg(target_arch = "wasm32")]
    guard: std::sync::RwLockWriteGuard<'a, T>,
}

impl<T> std::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> std::ops::DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

// =============================================================================
// Mutex - Platform-agnostic async Mutex
// =============================================================================

/// Platform-agnostic async mutex.
///
/// On native: Uses `tokio::sync::Mutex`
/// On WASM: Uses `std::sync::Mutex` (WASM is single-threaded)
pub struct Mutex<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::sync::Mutex<T>,
    #[cfg(target_arch = "wasm32")]
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    /// Create a new Mutex.
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            inner: tokio::sync::Mutex::new(value),
            #[cfg(target_arch = "wasm32")]
            inner: std::sync::Mutex::new(value),
        }
    }

    /// Acquire the lock.
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard {
            #[cfg(not(target_arch = "wasm32"))]
            guard: self.inner.lock().await,
            #[cfg(target_arch = "wasm32")]
            guard: self.inner.lock().unwrap(),
        }
    }
}

/// Guard for Mutex.
pub struct MutexGuard<'a, T> {
    #[cfg(not(target_arch = "wasm32"))]
    guard: tokio::sync::MutexGuard<'a, T>,
    #[cfg(target_arch = "wasm32")]
    guard: std::sync::MutexGuard<'a, T>,
}

impl<T> std::ops::Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> std::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}
