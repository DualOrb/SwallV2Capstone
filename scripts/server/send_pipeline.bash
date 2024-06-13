export port=5555
export L1=134.117.57.228
export L2=134.117.57.229
export L3=134.117.57.230
export L4=134.117.57.222

gst-launch-1.0 swall_compositor! videoconvert ! gtkwaylandsink
