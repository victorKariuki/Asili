//! UDP sockets: datagrams to and from any address, a default peer, broadcast and multicast.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use tokio::net::UdpSocket;

/// A bound UDP socket.
pub struct Udp {
    sock: UdpSocket,
}

/// The first address `addr` ("mwenyeji:mlango") resolves to, preferring `family`'s.
async fn resolve_one(addr: &str, prefer_v6: Option<bool>) -> Result<SocketAddr, String> {
    let (host, port) = crate::split_host_port(addr)?;
    let all: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| format!("kutafuta {host}: {e}"))?
        .collect();
    all.iter()
        .find(|a| prefer_v6.is_none_or(|v6| a.is_ipv6() == v6))
        .or(all.first())
        .copied()
        .ok_or_else(|| format!("kutafuta {host}: hakuna anwani"))
}

/// A UDP socket bound to `addr` (`"0.0.0.0:0"`: any address, any free port).
pub async fn udp_bind(addr: &str) -> Result<Udp, String> {
    let a = resolve_one(addr, None).await?;
    let sock = UdpSocket::bind(a)
        .await
        .map_err(|e| format!("udp {addr}: {e}"))?;
    Ok(Udp { sock })
}

impl Udp {
    fn family(&self) -> Option<bool> {
        self.sock.local_addr().ok().map(|a| a.is_ipv6())
    }

    /// Send one datagram to `addr`.
    pub async fn send_to(&self, data: &[u8], addr: &str) -> Result<(), String> {
        let to = resolve_one(addr, self.family()).await?;
        self.sock
            .send_to(data, to)
            .await
            .map(|_| ())
            .map_err(|e| format!("udp {to}: {e}"))
    }

    /// The next datagram (at most `max` bytes; the rest of a longer one is lost) and its sender.
    pub async fn recv_from(&self, max: usize) -> Result<(Vec<u8>, String), String> {
        let mut buf = vec![0u8; max];
        let (n, from) = self
            .sock
            .recv_from(&mut buf)
            .await
            .map_err(|e| format!("udp: {e}"))?;
        buf.truncate(n);
        Ok((buf, from.to_string()))
    }

    /// Make `addr` the default peer: `send` goes there, and only its datagrams arrive.
    pub async fn connect(&self, addr: &str) -> Result<(), String> {
        let to = resolve_one(addr, self.family()).await?;
        self.sock
            .connect(to)
            .await
            .map_err(|e| format!("udp {to}: {e}"))
    }

    /// Send one datagram to the default peer.
    pub async fn send(&self, data: &[u8]) -> Result<(), String> {
        self.sock
            .send(data)
            .await
            .map(|_| ())
            .map_err(|e| format!("udp: {e}"))
    }

    /// The address it is bound to.
    pub fn local_addr(&self) -> String {
        self.sock
            .local_addr()
            .map_or_else(|_| String::new(), |a| a.to_string())
    }

    /// Allow sending to broadcast addresses.
    pub fn set_broadcast(&self, on: bool) -> Result<(), String> {
        self.sock.set_broadcast(on).map_err(|e| format!("udp: {e}"))
    }

    /// Receive the multicast group `group`'s datagrams.
    pub fn join_multicast(&self, group: &str) -> Result<(), String> {
        let ip: IpAddr = group
            .parse()
            .map_err(|_| format!("kikundi batili: {group}"))?;
        match ip {
            IpAddr::V4(g) => self.sock.join_multicast_v4(g, Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(g) => self.sock.join_multicast_v6(&g, 0),
        }
        .map_err(|e| format!("udp kikundi {group}: {e}"))
    }
}
