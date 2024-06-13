#![deny(unused_crate_dependencies)]
use std::sync::{Arc, Mutex, Weak};

use tokio::runtime::Runtime;

static TOKIO_RUNTIME: Mutex<Weak<Runtime>> = Mutex::new(Weak::new());

/// Get the runtime singleton [Runtime] but let the runtime [drop] when no more [runtimes](Runtime) are left running.
pub fn get_tokio_runtime() -> Arc<Runtime> {
    let mut guard = TOKIO_RUNTIME.lock().unwrap();
    // The runtime is stored as a [Weak] so that it will get dropped when all instances are gone.
    if let Some(runtime) = guard.upgrade() {
        // Singleton is already created. Just return a copy
        runtime
    } else {
        // Singleton is empty. Create a new runtime and store it for future class.
        let runtime = Arc::new(Runtime::new().unwrap());
        *guard = Arc::downgrade(&runtime);
        runtime
    }
}
