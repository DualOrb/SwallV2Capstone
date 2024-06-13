#![deny(unused_crate_dependencies)]
use gstreamer::{glib, prelude::StaticType};

fn plugin_init(plugin: &gstreamer::Plugin) -> Result<(), glib::BoolError> {
    let filters = [
        (
            "print_navigation_events",
            swall_gst_event_capture::CaptureNav::static_type(),
        ),
        (
            "swall_compositor",
            swall_gst_compositor::CompositorRoot::static_type(),
        ),
        (
            "swall_input_client",
            swall_gst_input::InputClient::static_type(),
        ),
        (
            "swall_input_server",
            swall_gst_input::InputServer::static_type(),
        ),
    ];

    for (name, filter_type) in filters {
        gstreamer::Element::register(Some(plugin), name, gstreamer::Rank::NONE, filter_type)?;
    }

    Ok(())
}

gstreamer::plugin_define!(
    swall,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION")),
    "MIT",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    "https://carleton.ca"
);
