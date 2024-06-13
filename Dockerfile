# FROM rust:1.73-alpine3.18
FROM rust:1.76-bookworm

RUN rustup component add rustfmt

# Alpine packages
# benchmark_cmd         musl-dev
# rt-encoding           musl-dev gst-plugins-base-dev
# swall_compositor      musl-dev gst-plugins-base-dev eudev-dev libseat-dev
# swall_event_capture   musl-dev gst-plugins-base-dev
# swall_gst_compositor  musl-dev gst-plugins-base-dev eudev-dev libseat-dev
# wlr_capture           musl-dev gst-plugins-base-dev
# apk add musl-dev gst-plugins-base-dev eudev-dev libseat-dev

# Debian packages
# apps/idle             python3-pygame
# benchmark_cmd
# rt-encoding           libgstreamer-plugins-base1.0-dev
# swall_compositor      libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev
# swall_event_capture   libgstreamer-plugins-base1.0-dev
# swall_gst_compositor  libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev
# wlr_capture
# gst plugins           gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi gstreamer1.0-tools
# octranspo app         libwebkit2gtk-4.0-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev npm
# weston-terminal       weston
# patch-gtkwaylandsink  git meson libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgtk-3-dev

RUN --mount=target=/var/lib/apt/lists,type=cache,sharing=locked \
    --mount=target=/var/cache/apt,type=cache,sharing=locked \
    rm -f /etc/apt/apt.conf.d/docker-clean \
    && apt-get update -y \
    && apt-get install -y libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev weston \
    gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi gstreamer1.0-tools \
    libwebkit2gtk-4.0-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev npm \
    python3-pygame git meson libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgtk-3-dev
# TODO: Stop downloading gstreamer plugins as part of CI

# Optional script that makes development in the container easier
COPY --chmod=0755 ./dev-image/comfort.sh /root/comfort

# start patching gtkwaylandsink (taken from ./client.Dockerfile)
WORKDIR /root
RUN git clone --branch 1.22.10 --depth=1 --sparse https://gitlab.freedesktop.org/gstreamer/gstreamer.git \
    && cd /root/gstreamer \
    && git sparse-checkout set subprojects/gst-plugins-bad

WORKDIR /root/gstreamer/subprojects/gst-plugins-bad
ADD ./dev-image/*.patch /root/
RUN git apply /root/*.patch

RUN meson setup -Dauto_features=disabled -Dgtk3=enabled -Dwayland=enabled build \
    && ninja -C build \
    && cp build/ext/gtk/libgstgtkwayland.so /root/ \
    && cp build/ext/gtk/libgstgtkwayland.so /lib/x86_64-linux-gnu/gstreamer-1.0/libgstgtkwayland.so \
    && rm -r build
# end patching gtkwaylandsink

# Copy project into container
COPY ./bin /root/static/bin
COPY ./crates /root/static/crates
COPY ./Cargo.lock /root/static/Cargo.lock
COPY ./Cargo.toml /root/static/Cargo.toml


WORKDIR /root/static

# Build the app as part of the image build process
# RUN --mount=target=/home/.cargo/registry,type=cache,sharing=locked \
#     --mount=target=/home/.cargo/git,type=cache,sharing=locked \
#     cargo fmt --all --check && \
#     cargo check --all && \
#     cargo build --all --release

CMD cargo fmt --all --check && \
    cargo check --all && \
    cargo build --all --release