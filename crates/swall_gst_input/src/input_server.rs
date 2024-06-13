use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use futures::FutureExt;
use gstreamer::{
    glib::{
        self,
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    },
    subclass::{
        prelude::{ElementImpl, GstObjectImpl},
        ElementMetadata,
    },
    Caps, PadTemplate,
};
use gstreamer_base::subclass::{
    base_transform::{BaseTransformImpl, BaseTransformImplExt},
    BaseTransformMode,
};
use once_cell::sync::Lazy;
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    spawn,
    sync::{
        broadcast::{self, error::RecvError},
        oneshot, Mutex,
    },
};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

#[derive(Debug, Default)]
pub struct InputServer(Mutex<Option<InputServerInner>>);

#[derive(Debug)]
pub struct InputServerInner {
    /// Keeps the tokio runtime running while this object is live
    _tokio_rt: Arc<tokio::runtime::Runtime>,
    event_sender: broadcast::Sender<bytes::Bytes>,
    /// Dropping this will stop the listener tasks
    _close_sender: oneshot::Sender<()>,
}

#[glib::object_subclass]
impl ObjectSubclass for InputServer {
    const NAME: &'static str = "sWallInputServer";
    type Type = super::InputServer;
    type ParentType = gstreamer_base::BaseTransform;
    type Interfaces = ();
}

impl BaseTransformImpl for InputServer {
    // Passthru for better perf (I think)
    const MODE: BaseTransformMode = BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = true;

    const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

    fn src_event(&self, event: gstreamer::Event) -> bool {
        match event.view() {
            gstreamer::EventView::Navigation(nav) => {
                match crate::encoding::serialize_event(nav.structure().unwrap()) {
                    Ok(payload) => {
                        if let Some(this) = &mut *self.0.blocking_lock() {
                            // We can ignore errors since it just means the channel is closing
                            let _ = this.event_sender.send(payload);
                        }
                    }
                    Err(err) => {
                        println!("Error: {}", err)
                    }
                }
            }
            _ => (),
        }
        self.parent_src_event(event)
    }

    fn start(&self) -> Result<(), gstreamer::ErrorMessage> {
        let mut this = self.0.blocking_lock();
        if this.is_none() {
            let tokio_rt = swall_gst_tokio::get_tokio_runtime();

            // This notifies the socket listener to stop listening. That will drop both event senders,
            // which will cause the tcp streams tasks to close too (via RecvErr::Closed).
            let (close_sender, close_receiver) = oneshot::channel::<()>();

            let (event_sender, event_receiver) = broadcast::channel::<Bytes>(5);

            let event_receiver_factory = event_sender.clone();
            tokio_rt.spawn(async move {
                // Don't drop this until we stop listening so that the boardcast channel stays open event when no connects are present
                let _keep_event_receiver_alive = event_receiver;
                // TODO: Don't hardcode the port number
                let tcp_listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], crate::PORT)))
                    .await
                    .unwrap();

                let mut server_futures = TcpListenerStream::new(tcp_listener)
                    .map(Some)
                    .merge(close_receiver.into_stream().map(|_| None));
                while let Some(client_stream_result) = server_futures.next().await.flatten() {
                    let mut client_stream = client_stream_result.unwrap();

                    // Lowers latency by sending packets out immediately instead of accumulating them
                    client_stream.set_nodelay(true).unwrap();

                    let mut event_receiver = event_receiver_factory.subscribe();
                    spawn(async move {
                        loop {
                            match event_receiver.recv().await {
                                Err(RecvError::Closed) => break,
                                Err(RecvError::Lagged(_)) => { /* TODO: Log warning */ }
                                Ok(data) => {
                                    client_stream.write_all(&data).await.unwrap();
                                }
                            }
                        }
                    });
                }
            });

            *this = Some(InputServerInner {
                _tokio_rt: tokio_rt,
                event_sender,
                _close_sender: close_sender,
            });
        }

        self.parent_start()
    }

    fn stop(&self) -> Result<(), gstreamer::ErrorMessage> {
        let mut this = self.0.blocking_lock();
        *this = None;

        self.parent_stop()
    }
}

impl ElementImpl for InputServer {
    fn metadata() -> Option<&'static ElementMetadata> {
        static ELEMENT_METADATA: Lazy<ElementMetadata> = Lazy::new(|| {
            ElementMetadata::new(
                "sWall Navigation Event Capture",
                "sWall/Event",
                "Capture Upstream Navigation Events",
                "sWall Capstone Group",
            )
        });
        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gstreamer::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<PadTemplate>> = Lazy::new(|| {
            let src_caps = Caps::new_any();
            let src_pad = PadTemplate::new(
                "src",
                gstreamer::PadDirection::Src,
                gstreamer::PadPresence::Always,
                &src_caps,
            )
            .unwrap();

            let sink_caps = Caps::new_any();
            let sink_pad = PadTemplate::new(
                "sink",
                gstreamer::PadDirection::Sink,
                gstreamer::PadPresence::Always,
                &sink_caps,
            )
            .unwrap();
            vec![src_pad, sink_pad]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl GstObjectImpl for InputServer {}
impl ObjectImpl for InputServer {}
