#![deny(unused_crate_dependencies)]
use std::io::ErrorKind;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use gstreamer::{Buffer, Caps, ReferenceTimestampMeta};
pub use smithay::backend::input::ButtonState;
use smithay::{
    backend::{
        egl::{EGLContext, EGLDevice, EGLDisplay},
        renderer::{
            damage::OutputDamageTracker,
            element::{
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind,
            },
            gles::{GlesRenderer, GlesTexture},
            utils::on_commit_buffer_handler,
            ExportMem, Offscreen,
        },
    },
    delegate_compositor, delegate_seat, delegate_shm,
    input::{
        pointer::{ButtonEvent, MotionEvent, PointerHandle},
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        gbm::Format,
        wayland_protocols::xdg::{
            decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server::xdg_toplevel,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{
                wl_buffer, wl_seat,
                wl_surface::{self, WlSurface},
            },
            Client, Display,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, Size, Transform, SERIAL_COUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            self as smithay_compositor, with_surface_tree_downward, CompositorClientState,
            CompositorHandler, CompositorState, SurfaceAttributes, TraversalAction,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
    },
};
use tokio::{sync::oneshot, task};

mod compositor;
pub mod config;
mod controller;
pub mod util;

use crate::config::{AppConfig, CompositorConfig};
use crate::controller::{start_controller_socket, AppController};
use crate::util::ListeningSocket;

/// Hardcoded since [swall_gst_compositor] only supports one size
pub const HARDCODED_COMPOSITOR_SIZE: [u32; 2] = [4320, 1920];

impl BufferHandler for App {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfacePid(pub u32);

// Wayland protocol for creating new windows. Toplevel is a regular window. Pop-up is another option.
impl XdgShellHandler for App {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let client_pid = self
            .client_pid
            .expect("'wl_dispatch_intercept' not wrapping wayland dispatch event")
            .try_into()
            .unwrap();

        // TODO: Handle multiple windows
        let app_pos_opt = self
            .application_viewer
            .application_rect_by_pid_blocking(client_pid);

        let should_add_pid = surface.with_pending_state(|state| {
            if let Some(app_pos) = app_pos_opt {
                // TODO: Can we make this fullscreen to remove decorations? Probably needs to be configurable if it will later be resized.
                state.states.set(xdg_toplevel::State::Fullscreen);

                // Don't decorate the window. We do this because we want the windows too not have borders if possible.
                state.decoration_mode = Some(zxdg_toplevel_decoration_v1::Mode::ServerSide);

                // TODO: Set this?
                // state.bounds

                // Tell the window what size we want it to be
                state.size = Some(
                    (
                        app_pos.width.try_into().unwrap(),
                        app_pos.height.try_into().unwrap(),
                    )
                        .into(),
                );

                true
            } else {
                println!("Window created without associated process. Ignoring.");
                state.states.set(xdg_toplevel::State::Suspended);
                false
            }
        });

        if should_add_pid {
            // Attach the process id (pid) to the surface so that when rendering we can reference it to get the application position
            smithay_compositor::with_states(surface.wl_surface(), |x| {
                assert!(x
                    .data_map
                    .insert_if_missing_threadsafe(|| SurfacePid(client_pid)));
            });
        }

        // Sends all the state updates above to the wayland clients
        surface.send_configure();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // Handle popup creation here
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // Handle popup grab here
    }

    fn reposition_request(
        &mut self,
        _: smithay::wayland::shell::xdg::PopupSurface,
        _: PositionerState,
        _: u32,
    ) {
        // Handle reposition request
    }
}

// Compositor wayland protocol is required since this is a compositor. Smithay will handle the rest.
impl CompositorHandler for App {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
    }
}

// Shared memory wayland protocol must be implemented by default. Smithay will handle this for us.
impl ShmHandler for App {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

// Seats are waylands concepts for mouse + keyboard + screen + touch device combined.
impl SeatHandler for App {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

/// Root object that holds all compositor state.
pub(crate) struct App {
    compositor_state: CompositorState,

    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,

    seat: Seat<Self>,
    app_controller: Arc<AppController>,
    application_viewer: compositor::CompositorApplicationViewer,

    /// This should always be present inside [XdgShellHandler] methods.
    client_pid: Option<i32>,
}

impl App {
    pub(crate) async fn get_surface_at_pos(
        &self,
        location: Point<f64, Logical>,
    ) -> Option<(&WlSurface, Point<i32, Logical>)> {
        // Reversed since surfaces are rendered front to back so the later ones will be ontop
        for top_surface in self.xdg_shell_state.toplevel_surfaces().iter().rev() {
            let surface = top_surface.wl_surface();
            let app_config = smithay_compositor::with_states(surface, |surface_data| {
                surface_data.data_map.get::<SurfacePid>().unwrap().0
            });

            let Some(rect) = self
                .application_viewer
                .application_rect_by_pid(app_config)
                .await
            else {
                continue;
            };

            if rect.is_inside(location.x as u32, location.y as u32) {
                return Some((surface, Point::from((rect.x as i32, rect.y as i32))));
            }
        }
        None
    }

    /// [smithay] does not pass a [Client] to handler methods (such as [XdgShellHandler::new_toplevel]). To get
    /// around this we manually implement all the raw wayland handlers and "intercept" this argument manually by
    /// implementing in a bunch of traits manully in [custom_xdg_shell_impl] instead of with automatically with
    /// [smithay::delegate_xdg_shell]. Those manual implementation call this method.
    #[inline]
    pub(crate) fn wl_dispatch_intercept(&mut self, client: &Client, fun: impl FnOnce(&mut Self)) {
        let before = self.client_pid;
        let pid = client.get_data::<ClientState>().unwrap().client_pid;
        self.client_pid = Some(pid);

        fun(self);

        self.client_pid = before; // Restore to previous state (so that recursion is handled).
    }
}

pub struct Compositor {
    state: App,
    /// TODO: Shouldn't be in here. Move this out of the compositor
    launch_config: Vec<AppConfig>,
    gles_renderer: GlesRenderer,
    damage_tracker: OutputDamageTracker,
    texture: GlesTexture,
    start_time: std::time::Instant,
    reference_cap: Caps,
    size_buffer: Size<i32, Logical>,
    display: Display<App>,
    pointer: PointerHandle<App>,
    unix_socket_handle: oneshot::Receiver<std::io::Error>,

    /// Closes controller when compositor is dropped
    _controller_cancel_token: tokio::sync::broadcast::Sender<()>,
}

impl Compositor {
    pub async fn new(config: impl AsRef<CompositorConfig>) -> Result<Self> {
        let config = config.as_ref();
        // Setup all the handlers to the compositor
        let display: Display<App> = Display::new()?;
        let mut dh = display.handle();

        let compositor_state = CompositorState::new::<App>(&dh);
        let shm_state = ShmState::new::<App>(&dh, vec![]);

        // Make an input device "seat". Concept from the wayland protocol
        let mut seat_state = SeatState::new();
        let seat = seat_state.new_wl_seat(&dh, "winit");

        let compositor_app_handle = compositor::CompositorApplicationHandle::new();

        let app_controller = Arc::new(AppController::new(compositor_app_handle.clone()));

        // Build the root object that holds all wayland state so that it can be accessed from the callbacks
        let mut state = App {
            compositor_state,
            xdg_shell_state: XdgShellState::new::<App>(&dh),
            shm_state,
            seat_state,
            seat,
            app_controller,
            application_viewer: compositor_app_handle.view(),
            client_pid: None,
        };

        // Wayland's protocol communicates over a socket file. Applications will search in the folder specified in the
        // environment variable `XDG_RUNTIME_DIR` for `wayland-*` to initiate application windows. This accepts connections
        // from there. We create our own folder for the swall to have this socket in
        let wayland_socket: &Path = Path::new("/tmp/swall/wayland-0");

        // Create socket folder -> Clean fresh if already exists
        match std::fs::create_dir(wayland_socket.parent().unwrap()) {
            Ok(_t) => println!("Created Socket Folder"),
            Err(error) => match error.kind() {
                ErrorKind::AlreadyExists => {
                    println!("Socket folder already exists. Cleaning...");
                    for entry in std::fs::read_dir(wayland_socket.parent().unwrap())? {
                        let file_entry = entry?;
                        if !file_entry.metadata().unwrap().is_dir() {
                            std::fs::remove_file(file_entry.path())?;
                        }
                    }
                    println!("Cleaned all files in /tmp/swall");
                }
                _ => return Err(error)?,
            },
        }

        let wayland_listener =
            ListeningSocket::bind_absolute(wayland_socket.to_path_buf()).unwrap();

        let (mut unix_socket_error_sender, unix_socket_handle) =
            oneshot::channel::<std::io::Error>();

        // TODO: Accepting new clients should probably be done as part of app controller, also
        //       making associated files should be in there too (blocking on moving process spawning
        //       into app controller)
        tokio::spawn(async move {
            let tx_borrow = &mut unix_socket_error_sender;
            let future = async move {
                loop {
                    let stream = tokio::select! {
                        _ = tx_borrow.closed() => break Ok(()),
                        conn = wayland_listener.accept() => conn

                    };

                    let stream = stream?;

                    let client_cred = stream.peer_cred()?;
                    let client_pid = client_cred
                        .pid()
                        // This should basically always get optimized out since most platforms support pid ucred
                        .expect("Platform does not support pid ucred");

                    println!("Got a client: {:?} (pid: {client_pid})", stream);

                    dh.insert_client(
                        stream.into_std().unwrap(),
                        Arc::new(ClientState::new(client_pid)),
                    )?;
                }
            };
            let res = future.await;

            if let Err(error) = res {
                let _ = unix_socket_error_sender.send(error);
            }
        });

        // Start the app controller logic to send and receive state modifying commands
        let controller_cancel_token =
            start_controller_socket(state.app_controller.clone(), [config.width, config.height])
                .await?;
        println!("App Controller Successfully Started.");

        // TODO: Choose a device in a smarter way. Is the first one always the best? (maybe)
        // Create an opengl-es device for rendering frames on the gpu (or in software sometimes)
        let egl_devices = EGLDevice::enumerate()?.collect::<Vec<_>>();
        dbg!(&egl_devices);

        // SAFETY: Egldisplays are create with smithay so this is safe
        let egl_display = unsafe {
            EGLDisplay::new(
                egl_devices
                    .into_iter()
                    .last()
                    .ok_or_else(|| anyhow!("No EGL devices present"))?,
            )
        }?;
        let egl_context = EGLContext::new(&egl_display)?;

        // We need a screen since for the compositor canvas.
        // TODO: Don't hardcode width and height
        assert_eq!(
            config.width, HARDCODED_COMPOSITOR_SIZE[0],
            "Compositor only supports 'right' as width in config.json"
        );
        assert_eq!(
            config.height, HARDCODED_COMPOSITOR_SIZE[1],
            "Compositor only supports 'right' as height in config.json"
        );
        let size_buffer: Size<i32, Logical> = (config.width as i32, config.height as i32).into();
        let transform = Transform::Normal;

        // SAFETY: This context is not shared between threads because it was just created.
        let mut gles_renderer = unsafe { GlesRenderer::new(egl_context)? };
        let texture: GlesTexture = gles_renderer.create_buffer(
            Format::Abgr8888,
            size_buffer.to_buffer(1, Transform::Normal),
        )?;

        // Only stuff on the frame that has changed needs to be re-rendered. Wayland tracks this with the concept of "damage" portions of the screen.
        let damage_tracker = OutputDamageTracker::new(size_buffer.to_physical(1), 1.0, transform);

        // To send events to applications
        let pointer = state.seat.add_pointer();

        // Some applications expect a keyboard. Attach one as a dummy
        let _keyboard = state
            .seat
            .add_keyboard(Default::default(), 200, 200)
            .unwrap();

        Ok(Self {
            launch_config: config.launch.clone(),
            state,
            gles_renderer,
            damage_tracker,
            texture,
            start_time: std::time::Instant::now(),
            reference_cap: Caps::new_empty_simple("timestamp/duration"),
            size_buffer,
            display,
            pointer,
            unix_socket_handle,
            _controller_cancel_token: controller_cancel_token,
        })
    }

    // TODO: Use a more specific error type than [anyhow::Error]
    /// Ask the compositor to produce a single frame
    pub async fn generate_frame(&mut self) -> Result<Buffer> {
        // Bubble up errors from tasks
        if let Ok(error) = self.unix_socket_handle.try_recv() {
            return Err(error.into());
        }

        // Yield to tokio async executor so it can do other stuff
        // TODO: Don't do this if possible
        tokio::time::sleep(Duration::ZERO).await;

        // TODO: This shouldn't be in here at all
        if let Some(app_config) = self.launch_config.pop() {
            self.state
                .app_controller
                .spawn_process(&app_config)
                .await
                .unwrap();
        }

        // Collect windows (or surfaces in general like cursors) that need to be rendered.
        let top_level_surfaces = self.state.xdg_shell_state.toplevel_surfaces();
        let mut elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
            Vec::with_capacity(top_level_surfaces.len());
        for surface in top_level_surfaces {
            if let Some(surface_pid) =
                smithay_compositor::with_states(surface.wl_surface(), |surface_data| {
                    surface_data.data_map.get::<SurfacePid>().copied()
                })
            {
                if let Some(surface_area) = self
                    .state
                    .application_viewer
                    .application_rect_by_pid(surface_pid.0)
                    .await
                {
                    let e = render_elements_from_surface_tree(
                        &mut self.gles_renderer,
                        surface.wl_surface(),
                        (
                            surface_area.x.try_into().unwrap(),
                            surface_area.y.try_into().unwrap(),
                        ),
                        1.0,
                        1.0,
                        Kind::Unspecified,
                    );
                    elements.extend(e);
                } else {
                    // TODO: Add a warning here? This means there is a surface that is unaccounted for by spawning
                }
            } else {
                // TODO: Add a warning here? This means there is a surface that is unaccounted for by spawning
            }
        }

        // This will only re-render parts that have change. Setting age to zero will cause the whole screen to be rendered.
        let render_output = self.damage_tracker.render_output_with(
            &mut self.gles_renderer,
            self.texture.clone(),
            1, // TODO: Is this age correct? Seems like it?
            &elements,
            [0.67843137254, 0.141176, 0.2235294, 1.0], // Background colour
        )?;

        // Tell the surfaces that they're frame update request was handled
        for surface in self.state.xdg_shell_state.toplevel_surfaces() {
            send_frames_surface_tree(
                surface.wl_surface(),
                self.start_time.elapsed().as_millis() as u32,
            );
        }

        // Handle events from the wayland clients (applications)
        // 'block_in_place' informs tokio that we expect this might block (specifically because we blocking_lock on a tokio Mutex)
        task::block_in_place::<_, Result<()>>(|| {
            self.display.dispatch_clients(&mut self.state)?;
            self.display.flush_clients()?;
            Ok(())
        })?;

        // TODO: Do this threaded somehow to stop blocking?
        // Don't want half rendered frame
        render_output.sync.wait();

        // Pull a copy of the final frame into main memory so we can write it to a file (only for testing)
        let map = self.gles_renderer.copy_texture(
            &self.texture,
            Rectangle {
                loc: (0, 0).into(),
                size: (self.size_buffer.w, self.size_buffer.h).into(),
            },
            Format::Abgr8888,
        )?;

        let buffer_slice = self.gles_renderer.map_texture(&map)?;

        let mut buffer = Buffer::with_size(buffer_slice.len())?;
        buffer
            .get_mut()
            .unwrap()
            .copy_from_slice(0, buffer_slice)
            .unwrap();
        ReferenceTimestampMeta::add(
            buffer.get_mut().unwrap(),
            &self.reference_cap,
            self.start_time.elapsed().try_into().unwrap(),
            None,
        );

        Ok(buffer)
    }

    pub async fn send_pointer_button(&mut self, event: Event) {
        let time = (self.start_time.elapsed().as_millis() % (u32::MAX as u128)) as u32;

        match event {
            Event::Button {
                pointer_x,
                pointer_y,
                ..
            }
            | Event::Move {
                pointer_x,
                pointer_y,
            } => {
                let time = (self.start_time.elapsed().as_millis() % (u32::MAX as u128)) as u32;
                let location = Point::from((pointer_x, pointer_y));
                let event = MotionEvent {
                    location,
                    serial: SERIAL_COUNTER.next_serial(),
                    time,
                };

                let focus = self
                    .state
                    .get_surface_at_pos(location)
                    .await
                    .map(|(a, b)| (a.clone(), b));
                self.pointer.motion(&mut self.state, focus, &event);
            }
        }

        if let Event::Button { state, button, .. } = event {
            let event = ButtonEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time,
                button,
                state,
            };
            self.pointer.button(&mut self.state, &event)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Button {
        state: ButtonState,
        button: u32,
        pointer_x: f64,
        pointer_y: f64,
    },
    Move {
        pointer_x: f64,
        pointer_y: f64,
    },
}

pub fn send_frames_surface_tree(surface: &wl_surface::WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            // the surface may not have any user_data if it is a subsurface and has not
            // yet been commited
            for callback in states
                .cached_state
                .current::<SurfaceAttributes>()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

#[derive(Debug)]
struct ClientState {
    compositor_state: CompositorClientState,
    client_pid: i32,
}

impl ClientState {
    fn new(client_pid: i32) -> Self {
        Self {
            compositor_state: Default::default(),
            client_pid,
        }
    }
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        println!("initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        println!("disconnected");
    }
}

mod custom_xdg_shell_impl;

// Macros used to delegate protocol handling to types in the app state.
delegate_compositor!(App);
delegate_shm!(App);
delegate_seat!(App);
