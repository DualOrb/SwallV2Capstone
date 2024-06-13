use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use gstreamer::{
    glib::{
        self,
        subclass::{object::ObjectImpl, types::ObjectSubclass},
        value::ToValue,
        ParamSpecBuilderExt,
    },
    param_spec::GstParamSpecBuilderExt,
    subclass::{
        prelude::{ElementImpl, GstObjectImpl},
        ElementMetadata,
    },
    Caps, Event, PadTemplate,
};
use gstreamer_base::subclass::{
    base_transform::{BaseTransformImpl, BaseTransformImplExt},
    BaseTransformMode,
};
use once_cell::sync::Lazy;
use tokio::{net::TcpStream, sync::mpsc};

use crate::encoding::deserialize_event;

#[derive(Debug, Default)]
pub struct InputClient(Mutex<Option<InputClientInner>>, Mutex<InputClientSettings>);

#[derive(Debug)]
struct InputClientInner {
    /// Keeps the tokio runtime running while this object is live
    _tokio_rt: Arc<tokio::runtime::Runtime>,
    incoming_events: mpsc::Receiver<Event>,
}

#[derive(Debug, Default)]
struct InputClientSettings {
    config_file: Option<PathBuf>,
}

#[glib::object_subclass]
impl ObjectSubclass for InputClient {
    const NAME: &'static str = "sWallInputClient";
    type Type = super::InputClient;
    type ParentType = gstreamer_base::BaseTransform;
    type Interfaces = ();
}

impl BaseTransformImpl for InputClient {
    // Passthru for better perf (I think)
    const MODE: BaseTransformMode = BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = true;

    const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

    fn start(&self) -> Result<(), gstreamer::ErrorMessage> {
        let mut this = self.0.lock().unwrap();
        if this.is_none() {
            let tokio_rt = swall_gst_tokio::get_tokio_runtime();

            let e = self.1.lock().unwrap();
            let f = e
                .config_file
                .as_ref()
                .expect("Expected `config-file` option");

            let mut buf = String::new();
            File::open(f)
                .expect("Config file does not exists")
                .read_to_string(&mut buf)
                .unwrap();

            // TODO: Make sure the task gets cancelled
            let clients: Vec<crate::config::Client> =
                json5::from_str(&buf).expect("Deserialization error");

            let (event_sender, event_receiver) = mpsc::channel::<Event>(10);

            for client in clients {
                let event_sender = event_sender.clone();
                tokio_rt.spawn(async move {
                    const BACKOFF_DELAY_INITIAL: Duration = Duration::from_secs(1);
                    const BACKOFF_DELAY_MAX: Duration = Duration::from_secs(15);
                    const BACKOFF_DELAY_INCREMENT: Duration = Duration::from_secs(2);
                    let mut backoff_delay = BACKOFF_DELAY_INITIAL;
                    'retry: loop {
                        match TcpStream::connect((client.ip, crate::PORT)).await {
                            Err(err) => {
                                println!(
                                    "Client({}): backoff({}s), failed with: {}",
                                    client.ip,
                                    backoff_delay.as_secs(),
                                    err
                                );
                                tokio::time::sleep(backoff_delay).await;
                                backoff_delay =
                                    [BACKOFF_DELAY_MAX, backoff_delay + BACKOFF_DELAY_INCREMENT]
                                        .into_iter()
                                        .min()
                                        .unwrap();
                            }
                            Ok(tcp_stream) => {
                                backoff_delay = BACKOFF_DELAY_INITIAL; // Reset backoff
                                let fun = || async {
                                    // Need to move explicitly to make the borrowchecker happy
                                    let mut tcp_stream = tcp_stream;

                                    println!("Client({}): Connected.", client.ip);

                                    loop {
                                        let event =
                                            deserialize_event(&mut tcp_stream, client.x, client.y)
                                                .await?;
                                        if let Err(_) = event_sender.send(event).await {
                                            break Ok(());
                                        }
                                    }
                                };

                                let result: Result<(), anyhow::Error> = fun().await;
                                match result {
                                    Err(error) => {
                                        println!("Connection got error: {}", error);
                                        continue 'retry;
                                    }
                                    _ => break 'retry,
                                }
                            }
                        }
                    }
                });
            }

            *this = Some(InputClientInner {
                _tokio_rt: tokio_rt,
                incoming_events: event_receiver,
            });
        }

        self.parent_start()
    }

    fn stop(&self) -> Result<(), gstreamer::ErrorMessage> {
        let mut this = self.0.lock().unwrap();
        *this = None;

        self.parent_stop()
    }

    fn before_transform(&self, inbuf: &gstreamer::BufferRef) {
        if let Some(this) = &mut *self.0.lock().unwrap() {
            // Limit so that this doesn't hang
            for _ in 0..10 {
                match this.incoming_events.try_recv() {
                    Ok(event) => {
                        self.src_event(event);
                    }
                    Err(_) => break,
                }
            }
        }
        self.parent_before_transform(inbuf)
    }
}

impl ElementImpl for InputClient {
    fn metadata() -> Option<&'static ElementMetadata> {
        static ELEMENT_METADATA: Lazy<ElementMetadata> = Lazy::new(|| {
            ElementMetadata::new(
                "sWall Navigation Event Forward",
                "sWall/Event",
                "Forward Upstream Navigation Events",
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

impl GstObjectImpl for InputClient {}
impl ObjectImpl for InputClient {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpecString::builder("config-file")
                .nick("Config Location")
                .blurb("Location to a json file containing a list of remote input servers")
                .mutable_ready()
                .build()]
        });

        &PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "config-file" => {
                let value = value
                    .get::<String>()
                    .unwrap_or_else(|err| unreachable!("type checked upstream: {}", err));
                self.1.lock().unwrap().config_file = Some(PathBuf::from(value));
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "config-file" => self.1.lock().unwrap().config_file.to_value(),
            _ => unimplemented!(),
        }
    }
}
