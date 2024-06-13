//! The expansion of `delegate_xdg_shell!(App)` [smithay::delegate_xdg_shell] but
//! capture some parameters that aren't passed thru [smithay] (specifically [wayland_server::Client]).
//!
//! This is basically just the output of `cargo expand` but with cleaned up imports and calling [App::wl_dispatch_intercept].
use super::App;

use smithay::{
    reexports::{
        wayland_protocols::xdg::shell::server::{
            xdg_popup::XdgPopup, xdg_positioner::XdgPositioner, xdg_surface::XdgSurface,
            xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
        },
        wayland_server::{
            self, backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch,
            Resource,
        },
    },
    wayland::shell::xdg::{
        XdgPositionerUserData, XdgShellState, XdgShellSurfaceUserData, XdgSurfaceUserData,
        XdgWmBaseUserData,
    },
};

impl GlobalDispatch<XdgWmBase, ()> for App {
    fn bind(
        state: &mut Self,
        dhandle: &DisplayHandle,
        client: &Client,
        resource: wayland_server::New<XdgWmBase>,
        global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as GlobalDispatch<XdgWmBase, (), Self>>::bind(
                state,
                dhandle,
                client,
                resource,
                global_data,
                data_init,
            )
        })
    }
    fn can_view(client: Client, global_data: &()) -> bool {
        <XdgShellState as GlobalDispatch<XdgWmBase, (), Self>>::can_view(client, global_data)
    }
}
impl Dispatch<XdgWmBase, XdgWmBaseUserData> for App {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &XdgWmBase,
        request: <XdgWmBase as Resource>::Request,
        data: &XdgWmBaseUserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as Dispatch<XdgWmBase, XdgWmBaseUserData, Self>>::request(
                state, client, resource, request, data, dhandle, data_init,
            )
        })
    }
    fn destroyed(
        state: &mut Self,
        client: ClientId,
        resource: &XdgWmBase,
        data: &XdgWmBaseUserData,
    ) {
        <XdgShellState as Dispatch<XdgWmBase, XdgWmBaseUserData, Self>>::destroyed(
            state, client, resource, data,
        )
    }
}
impl Dispatch<XdgPositioner, XdgPositionerUserData> for App {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &XdgPositioner,
        request: <XdgPositioner as Resource>::Request,
        data: &XdgPositionerUserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as Dispatch<_, _, _>>::request(
                state, client, resource, request, data, dhandle, data_init,
            )
        })
    }
    fn destroyed(
        state: &mut Self,
        client: ClientId,
        resource: &XdgPositioner,
        data: &XdgPositionerUserData,
    ) {
        <XdgShellState as Dispatch<XdgPositioner, XdgPositionerUserData, Self>>::destroyed(
            state, client, resource, data,
        )
    }
}
impl Dispatch<XdgPopup, XdgShellSurfaceUserData> for App {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &XdgPopup,
        request: <XdgPopup as Resource>::Request,
        data: &XdgShellSurfaceUserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as Dispatch<XdgPopup, XdgShellSurfaceUserData, Self>>::request(
                state, client, resource, request, data, dhandle, data_init,
            )
        })
    }
    fn destroyed(
        state: &mut Self,
        client: ClientId,
        resource: &XdgPopup,
        data: &XdgShellSurfaceUserData,
    ) {
        <XdgShellState as Dispatch<XdgPopup, XdgShellSurfaceUserData, Self>>::destroyed(
            state, client, resource, data,
        )
    }
}
impl Dispatch<XdgSurface, XdgSurfaceUserData> for App {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &XdgSurface,
        request: <XdgSurface as Resource>::Request,
        data: &XdgSurfaceUserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as Dispatch<XdgSurface, XdgSurfaceUserData, Self>>::request(
                state, client, resource, request, data, dhandle, data_init,
            )
        })
    }
    fn destroyed(
        state: &mut Self,
        client: ClientId,
        resource: &XdgSurface,
        data: &XdgSurfaceUserData,
    ) {
        <XdgShellState as Dispatch<XdgSurface, XdgSurfaceUserData, Self>>::destroyed(
            state, client, resource, data,
        )
    }
}
impl Dispatch<XdgToplevel, XdgShellSurfaceUserData> for App {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &XdgToplevel,
        request: <XdgToplevel as Resource>::Request,
        data: &XdgShellSurfaceUserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        state.wl_dispatch_intercept(client, |state| {
            <XdgShellState as Dispatch<XdgToplevel, XdgShellSurfaceUserData, Self>>::request(
                state, client, resource, request, data, dhandle, data_init,
            )
        })
    }
    fn destroyed(
        state: &mut Self,
        client: ClientId,
        resource: &XdgToplevel,
        data: &XdgShellSurfaceUserData,
    ) {
        <XdgShellState as Dispatch<XdgToplevel, XdgShellSurfaceUserData, Self>>::destroyed(
            state, client, resource, data,
        )
    }
}
