use std::net::SocketAddr;

/// Obtain an address for use by a server. This will create a websocket
/// URL that tries to bind to all interfaces on port 50000.
pub fn default_server_address() -> url::Url {
    "ws://0.0.0.0:50000/".parse().unwrap()
}

/// Create a URL for use by client applications. This will create a websocket
/// URL that tries to connect to the localhost on port 50000.
pub fn default_client_address() -> url::Url {
    "ws://localhost:50000/".parse().unwrap()
}

/// Convert a URL to a socket address. This avoids doing DNS lookups at the
/// moment. Conversion follows approach as suggested in
/// [https://github.com/servo/rust-url/issues/530]
pub fn url_to_sockaddr(u: &url::Url) -> Option<SocketAddr> {
    let host = u.host()?;
    let port = u.port()?;

    let addr;

    match host {
        url::Host::Domain(_) => {
            return None;
        }
        url::Host::Ipv4(ip) => {
            addr = (ip, port).into();
            std::slice::from_ref(&addr)
        }
        url::Host::Ipv6(ip) => {
            addr = (ip, port).into();
            std::slice::from_ref(&addr)
        }
    };

    Some(addr)
}
