use std::net::SocketAddr;

// pub fn determine_ip_address() -> Option<String> {
//     local_ip_address::local_ip().map(|f| f.to_string()).ok()
// }

// pub fn default_local_ip_address() -> url::Url {
//     format!(
//         "ws://{}:50000/",
//         determine_ip_address().unwrap_or_else(|| "0.0.0.0".into())
//     )
//     .parse()
//     .unwrap()
// }

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
