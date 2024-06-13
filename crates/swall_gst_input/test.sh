gst-launch-1.0 -v --gst-plugin-path=../../target/debug/ videotestsrc ! swall_input_server ! gtkwaylandsink
gst-launch-1.0 -v --gst-plugin-path=../../target/debug/ videotestsrc ! swall_capture_nav ! swall_input_client config-file=client_config.json ! fakesink
