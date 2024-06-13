FROM debian:bookworm as gstreamer_build

RUN --mount=target=/var/lib/apt/lists,type=cache,sharing=locked \
    --mount=target=/var/cache/apt,type=cache,sharing=locked \
    rm -f /etc/apt/apt.conf.d/docker-clean \
    && apt-get update -y \
    && apt-get install -y git meson libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgtk-3-dev

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
    && rm -r build

FROM rust:1.76-bookworm as build

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
# benchmark_cmd
# rt-encoding           libgstreamer-plugins-base1.0-dev
# swall_compositor      libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev
# swall_event_capture   libgstreamer-plugins-base1.0-dev
# swall_gst_compositor  libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev
# wlr_capture
# Runtime plugins:      gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi gstreamer1.0-tools

RUN --mount=target=/var/lib/apt/lists,type=cache,sharing=locked \
    --mount=target=/var/cache/apt,type=cache,sharing=locked \
    rm -f /etc/apt/apt.conf.d/docker-clean \
    && apt-get update -y \
    && apt-get install -y libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev \
    gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi gstreamer1.0-tools
# TODO: Stop downloading gstreamer plugins as part of CI

# Optional script that makes development in the container easier
COPY --chmod=0755 ./dev-image/comfort.sh /root/comfort

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

RUN cargo fmt --all --check && \
    cargo build --package=swall_gst_plugin --release --locked

# Client Build - May change to something more simplistic
FROM rust:1.76-bookworm

RUN --mount=target=/var/lib/apt/lists,type=cache,sharing=locked \
    --mount=target=/var/cache/apt,type=cache,sharing=locked \
    rm -f /etc/apt/apt.conf.d/docker-clean \
    && apt-get update -y \
    && apt-get install -y libgstreamer-plugins-base1.0-dev libseat-dev libinput-dev libxkbcommon-dev \
    gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi gstreamer1.0-tools

# Make gstreamer plugin directory and add built project
RUN mkdir -p /gst-plugins
ENV GST_PLUGIN_PATH=/gst-plugins

COPY --from=gstreamer_build /root/libgstgtkwayland.so /gst-plugins
COPY --from=build /root/static/target/release/libswall.so /gst-plugins

# Default command / behaviour defined in compose file