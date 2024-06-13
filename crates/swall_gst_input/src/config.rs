use std::net::IpAddr;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Client {
    pub x: u32,
    pub y: u32,
    pub ip: IpAddr,
}
