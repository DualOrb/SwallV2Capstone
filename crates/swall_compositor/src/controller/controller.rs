use std::collections::HashMap;
use std::{path::Path, process::Command};

use anyhow::Result;

use tokio::sync::Mutex;

use crate::compositor::CompositorApplicationHandle;
use crate::config::{AppConfig, CompositorProcess, Rect};

#[derive(Debug)]
pub struct AppController {
    // TODO: Is it a good idea for this mutex to be in here?
    child_processes: Mutex<HashMap<u32, CompositorProcess>>,
    compositor_app_handle: CompositorApplicationHandle,
}

impl AppController {
    pub fn new(compositor_app_handle: CompositorApplicationHandle) -> Self {
        Self {
            child_processes: Default::default(),
            compositor_app_handle,
        }
    }

    /// Spawns a process from an [AppConfig] and redirects it's display variables to the swall Wayland Socket
    pub async fn spawn_process(&self, app_config: &AppConfig) -> Result<u32> {
        let wayland_socket: &Path = Path::new("/tmp/swall/wayland-0");

        let process = {
            // We need to lock this while spawning the process so there is no race condition between
            let mut positioner_guard = self.compositor_app_handle.reserve().await;

            println!("Spawning {}", app_config.executable);

            let process = Command::new(app_config.executable.as_str())
                .args(app_config.args.iter())
                .env("WAYLAND_DISPLAY", wayland_socket.file_name().unwrap())
                .env("XDG_RUNTIME_DIR", wayland_socket.parent().unwrap())
                .spawn()?;

            positioner_guard.set_application_position(process.id(), app_config.area);

            process
        };

        let pid = process.id();
        self.child_processes.lock().await.insert(
            pid,
            CompositorProcess {
                child: process,
                config: app_config.clone(),
            },
        );

        Ok(pid)
    }

    /// resizes a process window from a rect and updates the state
    pub async fn resize_process(&self, pid: &u32, rect: &Rect) -> Result<u32> {
        if let Some(compositor_process) = self.child_processes.lock().await.get_mut(pid) {
            compositor_process.config.area = *rect;
        } else {
            return Err(anyhow::anyhow!("Unknown pid {pid}"));
        }

        // Update the position inside the actual compositor
        self.compositor_app_handle
            .set_application_position(*pid, *rect)
            .await;

        Ok(*pid)
    }

    /// Kills a process by a u32 process Id
    /// Current processes spawned by the compositor can be obtained by using the [list_processes] function
    pub async fn kill_process(&self, pid: u32) -> Result<()> {
        if let Some(mut child_process) = self.child_processes.lock().await.remove(&pid) {
            child_process.child.kill()?;
            self.compositor_app_handle
                .remove_application_position(pid)
                .await;

            Ok(())
        } else {
            Err(anyhow::anyhow!("pid {pid} not found"))
        }
    }

    /// Lists the current processes managed by the compositor
    pub async fn list_processes(&self) -> Vec<(u32, AppConfig)> {
        self.child_processes
            .lock()
            .await
            .iter()
            .map(|(pid, process)| (*pid, process.config.clone()))
            .collect()
    }

    /// Returns the dimensions of the overall compositor canvas
    pub async fn send_screen_size(screen_size: [u32; 2]) -> [u32; 2] {
        // TODO: Remove this? Store screen_size in app controller obj
        screen_size
    }
}
