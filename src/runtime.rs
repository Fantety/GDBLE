use std::sync::Arc;
use tokio::runtime::Runtime;

/// RuntimeManager manages the Tokio async runtime for executing BLE operations
///
/// This struct provides a centralized way to manage the Tokio runtime lifecycle
/// and execute async tasks. It ensures that all async operations use the same
/// runtime instance, preventing resource conflicts.
pub struct RuntimeManager {
    runtime: Arc<Runtime>,
}

impl RuntimeManager {
    /// Creates a new RuntimeManager with a multi-threaded Tokio runtime
    ///
    /// # Returns
    /// A new RuntimeManager instance with an initialized runtime
    ///
    /// # Panics
    /// Panics if the Tokio runtime cannot be created
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        Self {
            runtime: Arc::new(runtime),
        }
    }

    /// Returns a cloned Arc reference to the underlying Tokio runtime
    ///
    /// This allows sharing the runtime across multiple components while
    /// maintaining thread safety through Arc's reference counting.
    ///
    /// # Returns
    /// An Arc-wrapped reference to the Runtime
    pub fn runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }

    /// Spawns an async task on the runtime
    ///
    /// This method schedules a future to run on the Tokio runtime's thread pool.
    /// The task runs independently and its result is not directly returned.
    ///
    /// # Type Parameters
    /// * `F` - A Future that is Send and has a 'static lifetime
    ///
    /// # Parameters
    /// * `future` - The async task to execute
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let _ = self.runtime.spawn(future);
    }

    /// Blocks the current thread until the future completes
    ///
    /// This method should be used sparingly as it blocks the calling thread.
    /// Prefer using spawn() for non-blocking execution when possible.
    ///
    /// # Type Parameters
    /// * `F` - A Future type
    ///
    /// # Parameters
    /// * `future` - The async task to execute and wait for
    ///
    /// # Returns
    /// The output of the completed future
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_runtime_manager_creation() {
        let manager = RuntimeManager::new();
        assert!(Arc::strong_count(&manager.runtime) == 1);
    }

    #[test]
    fn test_runtime_clone() {
        let manager = RuntimeManager::new();
        let runtime1 = manager.runtime();
        let runtime2 = manager.runtime();

        // Both should point to the same runtime
        assert!(Arc::ptr_eq(&runtime1, &runtime2));
    }

    #[test]
    fn test_spawn_execution() {
        let manager = RuntimeManager::new();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        manager.spawn(async move {
            flag_clone.store(true, Ordering::SeqCst);
        });

        // Give the task time to execute
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_block_on_execution() {
        let manager = RuntimeManager::new();

        let result = manager.block_on(async { 42 });

        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_with_computation() {
        let manager = RuntimeManager::new();

        let result = manager.block_on(async {
            let mut sum = 0;
            for i in 1..=10 {
                sum += i;
            }
            sum
        });

        assert_eq!(result, 55);
    }
}
