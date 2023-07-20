use std::net::SocketAddr;

pub fn default_server_address() -> url::Url {
    "ws://0.0.0.0:50000/".parse().unwrap()
}

pub fn default_client_address() -> url::Url {
    "ws://localhost:50000/".parse().unwrap()
}

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
