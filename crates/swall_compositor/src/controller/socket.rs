use std::{path::Path, sync::Arc};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::UnixListener;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    spawn,
    sync::broadcast,
};
use tokio_stream::{
    wrappers::{BroadcastStream, SplitStream, UnixListenerStream},
    StreamExt,
};

use crate::config::{AppConfig, AppControllerCommand};

use super::AppController;

pub const MSG_SPLITTER: u8 = 0x1e;

/// Starts the app controller for the swall compositor
/// Implements a receive -> process -> respond loop to process data
/// Data is modified through the shared state struct
pub async fn start_controller_socket(
    inner_state: Arc<AppController>,
    screen_size: [u32; 2],
) -> Result<broadcast::Sender<()>> {
    // Create the socket in which the controller will receive commands over
    let app_controller_socket: &Path = Path::new("/tmp/swall/control-0");
    let control_listener = UnixListener::bind(app_controller_socket.to_path_buf())?;
    println!("Controller Socket Created");

    // TODO: Use tokio watch instead of broadcast here?
    let (controller_canceller, controller_cancel) = broadcast::channel::<()>(1);

    // Main receive -> process -> respond loop
    spawn(async move {
        println!("Ready to receive controller input");

        enum ControlEvent {
            NewStream(Result<UnixStream, std::io::Error>),
            Cancel,
        }

        // Accept new connections but also see if the task gets cancelled
        let mut control_stream = UnixListenerStream::new(control_listener)
            .map(ControlEvent::NewStream)
            .merge(
                BroadcastStream::new(controller_cancel.resubscribe()).map(|_| ControlEvent::Cancel),
            );

        while let Some(control_event) = control_stream.next().await {
            match control_event {
                ControlEvent::NewStream(Ok(new_stream)) => {
                    let inner_state = inner_state.clone();
                    let controller_cancel = controller_cancel.resubscribe();
                    spawn(async move {
                        let (stream_reader, mut stream_writer) = new_stream.into_split();
                        let stream_reader = BufReader::new(stream_reader);

                        enum ControlEvent {
                            NewSegment(Result<Vec<u8>, std::io::Error>),
                            Cancel,
                        }
                        let mut segments = SplitStream::new(stream_reader.split(MSG_SPLITTER))
                            .map(ControlEvent::NewSegment)
                            .merge(
                                BroadcastStream::new(controller_cancel)
                                    .map(|_| ControlEvent::Cancel),
                            );

                        while let Some(control_event) = segments.next().await {
                            match control_event {
                                ControlEvent::NewSegment(segment) => {
                                    let segment = segment.unwrap();
                                    let message = std::str::from_utf8(&segment).unwrap();
                                    println!("Read from controller: {}", message);
                                    let response =
                                        process_command(&message, &inner_state, screen_size).await;

                                    stream_writer.write_all(response.as_bytes()).await.unwrap();
                                    stream_writer.write_all(&[MSG_SPLITTER]).await.unwrap();
                                    println!("Response sent {}", response);
                                }
                                ControlEvent::Cancel => return,
                            }
                        }
                    });
                }
                ControlEvent::NewStream(Err(socket_err)) => {
                    println!("Failed to accept connection: {}", socket_err)
                }
                ControlEvent::Cancel => return,
            }
        }
    });

    Ok(controller_canceller)
}

/// Processes a command string (JSON) of format [CompositorAction] and takes appropriate action
pub(crate) async fn process_command(
    command: &str,
    app_controller: impl AsRef<AppController>,
    screen_size: [u32; 2],
) -> String {
    let compositor_action: AppControllerCommand = match serde_json::from_str(command) {
        Ok(result) => result,
        Err(error) => {
            return json!(AppControllerResponse {
                success: false,
                pid: None,
                screen_width: None,
                screen_height: None,
                config: None,
                process_ids: None,
                error: format!("{}", error).into(),
            })
            .to_string()
        }
    };

    let app_controller = app_controller.as_ref();
    let res = match compositor_action {
        AppControllerCommand::Spawn { config } => {
            app_controller.spawn_process(&config).await.map(|pid| {
                json!(AppControllerResponse {
                    success: true,
                    pid: Some(pid),
                    screen_width: None,
                    screen_height: None,
                    config: Some(config),
                    process_ids: None,
                    error: None
                })
                .to_string()
            })
        }
        AppControllerCommand::Move { pid, rect } => {
            app_controller.resize_process(&pid, &rect).await.map(|pid| {
                json!(AppControllerResponse {
                    success: true,
                    pid: Some(pid),
                    screen_width: None,
                    screen_height: None,
                    config: None,
                    process_ids: None,
                    error: None
                })
                .to_string()
            })
        }
        AppControllerCommand::Kill { pid } => app_controller.kill_process(pid).await.map(|_| {
            json!(AppControllerResponse {
                success: true,
                pid: Some(pid),
                screen_width: None,
                screen_height: None,
                config: None,
                process_ids: None,
                error: None
            })
            .to_string()
        }),
        AppControllerCommand::List => {
            let configs = app_controller.list_processes().await;

            Ok(json!(AppControllerResponse {
                success: true,
                pid: None,
                screen_width: None,
                screen_height: None,
                config: None,
                process_ids: Some(configs),
                error: None
            })
            .to_string())
        }
        AppControllerCommand::ScreenSize => {
            let screen_size = AppController::send_screen_size(screen_size).await;
            Ok(json!(AppControllerResponse {
                success: true,
                pid: None,
                screen_width: Some(screen_size[0]),
                screen_height: Some(screen_size[1]),
                config: None,
                process_ids: None,
                error: None
            })
            .to_string())
        }
    };

    let response = match res {
        Ok(msg) => msg,
        Err(error) => json!(AppControllerResponse {
            success: false,
            pid: None,
            screen_width: None,
            screen_height: None,
            config: None,
            process_ids: None,
            error: format!("{}", error).into(),
        })
        .to_string(),
    };

    return response;
}

// TODO: Move this into swall_compositor_config
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AppControllerResponse {
    pub success: bool,
    pub pid: Option<u32>,
    pub screen_width: Option<u32>,
    pub screen_height: Option<u32>,
    pub config: Option<AppConfig>,
    pub process_ids: Option<Vec<(u32, AppConfig)>>,
    pub error: Option<String>,
}
