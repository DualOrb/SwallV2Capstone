#![deny(unused_crate_dependencies)]
use gstreamer::glib;

mod imp {
    use gstreamer::subclass::prelude::{ElementImpl, GstObjectImpl, ObjectImpl, ObjectSubclass};
    use gstreamer::subclass::ElementMetadata;
    use gstreamer::{glib, Caps, PadTemplate};
    use gstreamer_base::subclass::prelude::{BaseTransformImpl, BaseTransformImplExt};
    use gstreamer_base::subclass::BaseTransformMode;
    use once_cell::sync::Lazy;

    /// Capturing the navigation events
    #[derive(Debug, Default)]
    pub struct CaptureNav {}

    #[glib::object_subclass]
    impl ObjectSubclass for CaptureNav {
        const NAME: &'static str = "sWallPrintNavigationEvents";
        type Type = super::CaptureNav;
        type ParentType = gstreamer_base::BaseTransform;
        type Interfaces = ();
    }

    impl BaseTransformImpl for CaptureNav {
        // Passthru for better perf (I think)
        const MODE: BaseTransformMode = BaseTransformMode::AlwaysInPlace;
        const PASSTHROUGH_ON_SAME_CAPS: bool = true;

        const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

        fn src_event(&self, event: gstreamer::Event) -> bool {
            match event.type_() {
                gstreamer::EventType::Navigation => {
                    // Navigation events can be captured here...
                    dbg!(&event);
                }
                _ => {}
            }
            self.parent_src_event(event)
        }
    }

    impl ElementImpl for CaptureNav {
        fn metadata() -> Option<&'static ElementMetadata> {
            static ELEMENT_METADATA: Lazy<ElementMetadata> = Lazy::new(|| {
                ElementMetadata::new(
                    "Print Navigation Events",
                    "sWall/Event",
                    "Print navigation events to stdout",
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

    impl GstObjectImpl for CaptureNav {}
    impl ObjectImpl for CaptureNav {}
}

glib::wrapper! {
    pub struct CaptureNav(ObjectSubclass<imp::CaptureNav>) @extends gstreamer_base::BaseTransform, gstreamer::Element, gstreamer::Object;
}
