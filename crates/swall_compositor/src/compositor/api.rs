pub(crate) mod application {
    use std::{collections::HashMap, sync::Arc};

    use tokio::sync::{Mutex, MutexGuard};

    use crate::config;

    type Inner = Arc<Mutex<HashMap<u32, config::Rect>>>;

    /// Handle that can be used to query the position of an application. Used
    /// internally by the compositor.
    #[derive(Debug, Clone)]
    pub struct CompositorApplicationViewer(Inner);

    impl CompositorApplicationViewer {
        pub async fn application_rect_by_pid(&self, pid: u32) -> Option<config::Rect> {
            self.0.lock().await.get(&pid).copied()
        }

        pub fn application_rect_by_pid_blocking(&self, pid: u32) -> Option<config::Rect> {
            self.0.blocking_lock().get(&pid).copied()
        }
    }

    /// Used by the client to inform the compositor were applications should be
    /// positioned. Think of this as the "write" half of [CompositorApplicationViewer].
    #[derive(Debug, Clone)]
    pub struct CompositorApplicationHandle(Inner);

    impl CompositorApplicationHandle {
        pub fn new() -> Self {
            Self(Default::default())
        }

        /// Create a new connection [CompositorApplicationViewer]
        pub fn view(&self) -> CompositorApplicationViewer {
            CompositorApplicationViewer(self.0.clone())
        }

        /// Set where the compositor should position an application on the global rendering canvas
        pub async fn set_application_position(&self, pid: u32, rect: config::Rect) {
            self.0.lock().await.insert(pid, rect);
        }

        /// Set where the compositor should position an application on the global rendering canvas
        pub async fn reserve(&self) -> PositionSetterGuard {
            PositionSetterGuard(self.0.lock().await)
        }

        /// Stop rendering an application on the global canvas
        pub async fn remove_application_position(&self, pid: u32) -> bool {
            self.0.lock().await.remove(&pid).is_some()
        }
    }

    /// Locks all the viewers temporarily while the position is being set. This is necessary since
    /// to get the pid of a process you need to spawn the process, which will try to view the position.
    #[derive(Debug)]
    pub struct PositionSetterGuard<'a>(MutexGuard<'a, HashMap<u32, config::Rect>>);

    impl<'a> PositionSetterGuard<'a> {
        /// Set where the compositor should position an application on the global rendering canvas
        pub fn set_application_position(&mut self, pid: u32, rect: config::Rect) {
            self.0.insert(pid, rect);
        }
    }
}
