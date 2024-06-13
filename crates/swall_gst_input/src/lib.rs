#![deny(unused_crate_dependencies)]
pub(crate) mod config;
pub(crate) mod encoding;
mod input_client;
mod input_server;

pub const PORT: u16 = 6503;

gstreamer::glib::wrapper! {
    pub struct InputClient(ObjectSubclass<input_client::InputClient>) @extends gstreamer_base::BaseTransform, gstreamer::Element, gstreamer::Object;
}

gstreamer::glib::wrapper! {
    pub struct InputServer(ObjectSubclass<input_server::InputServer>) @extends gstreamer_base::BaseTransform, gstreamer::Element, gstreamer::Object;
}
