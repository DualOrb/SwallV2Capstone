#![deny(unused_crate_dependencies)]
use gstreamer::glib;

pub(crate) mod translate_event;

mod imp {
    use gstreamer::glib::subclass::object::ObjectImplExt;
    use gstreamer::glib::subclass::types::ObjectSubclassExt;
    use gstreamer::subclass::prelude::{ElementImpl, GstObjectImpl, ObjectImpl, ObjectSubclass};
    use gstreamer::subclass::ElementMetadata;
    use gstreamer::{glib, Buffer, Caps, Event, EventType, PadTemplate};
    use gstreamer_base::prelude::BaseSrcExt;
    use gstreamer_base::subclass::base_src::{BaseSrcImpl, CreateSuccess};
    use gstreamer_base::subclass::prelude::PushSrcImpl;
    use once_cell::sync::Lazy;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use swall_compositor::{config::CompositorConfig, Compositor, HARDCODED_COMPOSITOR_SIZE};
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;
    use tokio::task::LocalSet;

    use crate::translate_event::translate_event;

    /// Capturing the navigation events
    #[derive(Debug)]
    pub struct CompositorRoot {
        tokio_rt: Arc<tokio::runtime::Runtime>,
        config: Arc<CompositorConfig>,
        compositor_notifiers: Mutex<Option<(mpsc::Receiver<Buffer>, mpsc::Sender<Event>)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CompositorRoot {
        const NAME: &'static str = "sWallCompositor";
        type Type = super::CompositorRoot;
        type ParentType = gstreamer_base::PushSrc;
        type Interfaces = ();

        fn new() -> Self {
            let config_define = || -> anyhow::Result<CompositorConfig> {
                let config_file = Path::new("compositor_config.json");

                let mut buf = String::new();
                File::open(config_file)
                    .map_err(|err| match err.kind() {
                        std::io::ErrorKind::NotFound => anyhow::anyhow!(
                            "'{}' is missing in current directory",
                            config_file.display(),
                        ),
                        _ => err.into(),
                    })?
                    .read_to_string(&mut buf)?;

                let config: CompositorConfig = json5::from_str(&buf)?;

                Ok(config)
            };

            let config = config_define().unwrap();

            let tokio_rt = swall_gst_tokio::get_tokio_runtime();
            Self {
                tokio_rt,
                config: config.into(),
                compositor_notifiers: Mutex::new(None),
            }
        }
    }

    impl PushSrcImpl for CompositorRoot {
        fn create(
            &self,
            _buffer: Option<&mut gstreamer::BufferRef>,
        ) -> std::result::Result<CreateSuccess, gstreamer::FlowError> {
            let buffer = self
                .compositor_notifiers
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .0
                .blocking_recv()
                .expect("Compositor stopped unexpectedly. The compositor probably panicked");
            Ok(CreateSuccess::NewBuffer(buffer))
        }
    }

    impl BaseSrcImpl for CompositorRoot {
        fn event(&self, event: &gstreamer::Event) -> bool {
            match event.type_() {
                EventType::Navigation => {
                    if let Some((_, event_sender)) = &mut *self.compositor_notifiers.lock().unwrap()
                    {
                        let _ = event_sender.blocking_send(event.to_owned()); // TODO: This can cause a deadlock because it blocks
                    }
                }
                _ => {}
            }
            true
        }

        fn start(&self) -> Result<(), gstreamer::ErrorMessage> {
            let mut notifiers = self.compositor_notifiers.lock().unwrap();

            if notifiers.is_some() {
                return Ok(());
            }

            let (frame_sender, frame_receiver) = mpsc::channel(1);
            let (event_sender, mut event_receiver) = mpsc::channel::<Event>(4);

            let config = self.config.clone();
            let rt = self.tokio_rt.clone();
            // Spawn off a thread generating compositor frames
            // The compositor is not [Send] so in order to make it async we need to tell the executor
            // to only run the task on a single thread. This is what `spawn_local` refers to.
            thread::spawn(move || {
                let local = LocalSet::new();

                local.spawn_local(async move {
                    // TODO: Better Error Handling
                    let mut compositor = Compositor::new(config).await.unwrap();

                    'compositor_loop: loop {

                        // Reserve space for a frame in the channel while continuing to forward events.
                        // This prevents deadlocks between the compositor and gstreamer plugin tasks.
                        let frame_slot = {
                            // Using the same reserver future between slot reserver loops guarantees
                            // that we don't loss our spot in the queue (or 'fairness').
                            let frame_sender_reserver_future = frame_sender.reserve();
                            tokio::pin!(frame_sender_reserver_future);

                            let mut event_received_while_waiting = None;
                            'slot_reserver_loop: loop {

                                // This limits the number of events that can be processed before we check our
                                // frame reservation to prevents the senario where lots of events are present so
                                // our frame reservation isn't checked. Put another way, this guarantess that the
                                // frame channel advances.
                                let event_iter = event_received_while_waiting
                                    .take()
                                    .into_iter()
                                    .chain((0..=5).map(|_| event_receiver.try_recv()));

                                for event_result in event_iter {
                                    let event = match event_result {
                                        Ok(event) => event,
                                        Err(TryRecvError::Empty) => break,
                                        Err(TryRecvError::Disconnected) => break 'compositor_loop,
                                    };

                                    match translate_event(event) {
                                        Ok(event) => {
                                            compositor.send_pointer_button(event).await;
                                        }
                                        Err(error) => println!("Error: {error}"),
                                    };
                                }

                                // Proceed to frame reserving that continues to forward events. Not forwarding
                                // events while we wait for the next frame slot can cause deadlocks because
                                // when the frame and event channels are full they create a ciruclar waiting
                                // dependency on each other.
                                break tokio::select! {
                                    // Checking for reservations before checking for events is important to
                                    // ensure frame progression. Event progression is already guaranteed since
                                    // we check it manually above. 'biased' makes the polling order sequential.
                                    biased;

                                    slot = &mut frame_sender_reserver_future => {
                                        let Ok(slot) = slot else {
                                            break 'compositor_loop
                                        };
                                        slot
                                    }
                                    event = event_receiver.recv() => {
                                        event_received_while_waiting = Some(event.ok_or(TryRecvError::Disconnected));
                                        continue 'slot_reserver_loop
                                    }
                                };
                            }
                        };

                        let buffer = compositor.generate_frame().await.unwrap();
                        frame_slot.send(buffer);
                    }
                });

                rt.block_on(local); // Run the task on this thread
            });

            *notifiers = Some((frame_receiver, event_sender));

            Ok(())
        }

        fn stop(&self) -> Result<(), gstreamer::ErrorMessage> {
            // This will destroy the channel between the plugin in compositor task, stopping the task
            *self.compositor_notifiers.lock().unwrap() = None;
            Ok(())
        }
    }

    impl ElementImpl for CompositorRoot {
        fn metadata() -> Option<&'static ElementMetadata> {
            static ELEMENT_METADATA: Lazy<ElementMetadata> = Lazy::new(|| {
                ElementMetadata::new(
                    "sWall Compositor Capture",
                    "sWall_compositor/Capture",
                    "Wayland compositor server video source",
                    "sWall Capstone Group",
                )
            });
            Some(&*ELEMENT_METADATA)
        }

        fn pad_templates() -> &'static [gstreamer::PadTemplate] {
            static PAD_TEMPLATES: Lazy<Vec<PadTemplate>> = Lazy::new(|| {
                let src_caps = Caps::builder("video/x-raw")
                    .field("format", "RGBA")
                    .field("width", HARDCODED_COMPOSITOR_SIZE[0] as i32) // TODO: How to support ranges? Also don't hardcode pls (make dynamic)?
                    .field("height", HARDCODED_COMPOSITOR_SIZE[1] as i32) // TODO: How to support ranges?
                    .build();
                let src_pad = PadTemplate::new(
                    "src",
                    gstreamer::PadDirection::Src,
                    gstreamer::PadPresence::Always,
                    &src_caps,
                )
                .unwrap();

                vec![src_pad]
            });

            PAD_TEMPLATES.as_ref()
        }
    }

    impl GstObjectImpl for CompositorRoot {}
    impl ObjectImpl for CompositorRoot {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().set_live(true);
            self.obj().set_format(gstreamer::Format::Time);
        }
    }
}

glib::wrapper! {
    pub struct CompositorRoot(ObjectSubclass<imp::CompositorRoot>) @extends gstreamer_base::PushSrc, gstreamer_base::BaseSrc, gstreamer::Element, gstreamer::Object;
}
