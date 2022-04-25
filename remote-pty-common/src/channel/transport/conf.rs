use std::{
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
};

#[derive(Debug, PartialEq)]
pub enum TransportType {
    Unix(String),
    Tcp(SocketAddr),
}

impl FromStr for TransportType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("tcp:") {
            return Ok(Self::Tcp(
                s[4..]
                    .to_socket_addrs()
                    .map_err(|err| format!("failed to parse {} into socket address: {}", s, err))?
                    .next()
                    .unwrap(),
            ));
        }

        if s.starts_with("unix:") {
            return Ok(Self::Unix(s[5..].to_string()));
        }

        Err("unknown transport spec (tcp:0.0.0.0:1234 | unix:/path) supported".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, SocketAddr, SocketAddrV4},
        str::FromStr,
    };

    use super::TransportType;

    #[test]
    fn test_parse_tcp() {
        assert_eq!(
            TransportType::from_str("tcp:127.0.0.1:1234"),
            Ok(TransportType::Tcp(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::LOCALHOST,
                1234
            ))))
        );
    }

    #[test]
    fn test_parse_unix() {
        assert_eq!(
            TransportType::from_str("unix:/test/path"),
            Ok(TransportType::Unix("/test/path".to_string()))
        );
    }

    #[test]
    fn test_invalid() {
        assert_eq!(TransportType::from_str("who knows").is_err(), true);
    }
}
