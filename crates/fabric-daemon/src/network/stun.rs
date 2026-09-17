//! STUN NAT traversal client (RFC 5389).
//!
//! Discovers external addresses and performs UDP hole-punching for P2P connectivity.

use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::time::{timeout, Duration};

use super::NetworkError;

// RFC 5389 constants.
const MAGIC_COOKIE: u32 = 0x2112A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;
const HEADER_SIZE: usize = 20;
const ATTR_HDR: usize = 4;
const ATTR_MAPPED: u16 = 0x0001;
const ATTR_XOR_MAPPED: u16 = 0x0020;
const ATTR_CHANGED: u16 = 0x0004;
const TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_RETRIES: u32 = 3;

/// Well-known public STUN servers.
pub const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
    "stun3.l.google.com:19302",
    "stun4.l.google.com:19302",
    "stun.ekiga.net:3478",
    "stun.ideasip.com:3478",
    "stun.schlund.de:3478",
    "stun.voiparound.com:3478",
    "stun.voipbuster.com:3478",
    "stun.voipstunt.com:3478",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StunTransport {
    Udp,
    Tcp,
}

/// Result of a successful STUN binding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StunResponse {
    pub external_addr: SocketAddr,
    pub mapped_addr: SocketAddr,
    pub changed_addr: Option<SocketAddr>,
    pub server: String,
}

/// Configuration for STUN binding requests.
#[derive(Debug, Clone)]
pub struct StunBindConfig {
    pub transport: StunTransport,
    pub retries: u32,
    pub timeout: Duration,
}

impl Default for StunBindConfig {
    fn default() -> Self {
        Self {
            transport: StunTransport::Udp,
            retries: DEFAULT_RETRIES,
            timeout: TIMEOUT,
        }
    }
}

/// STUN client for discovering external addresses and hole-punching.
#[derive(Debug, Default, Clone)]
pub struct StunClient {
    servers: Vec<String>,
    config: StunBindConfig,
}

impl StunClient {
    pub fn new() -> Self {
        Self {
            servers: DEFAULT_STUN_SERVERS.iter().map(|s| s.to_string()).collect(),
            config: StunBindConfig::default(),
        }
    }

    pub fn with_servers(servers: Vec<String>) -> Self {
        Self {
            servers,
            config: StunBindConfig::default(),
        }
    }

    pub fn with_config(config: StunBindConfig) -> Self {
        Self {
            servers: DEFAULT_STUN_SERVERS.iter().map(|s| s.to_string()).collect(),
            config,
        }
    }

    /// Queries the first reachable STUN server for the external address.
    pub async fn query_external_address(&self) -> Result<StunResponse, NetworkError> {
        self.query_with_transport(self.config.transport).await
    }

    /// Queries STUN servers using a specific transport.
    pub async fn query_with_transport(
        &self,
        transport: StunTransport,
    ) -> Result<StunResponse, NetworkError> {
        let mut last_error = None;
        for server in &self.servers {
            match self.send_binding_request(server, transport).await {
                Ok(r) => return Ok(r),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error.unwrap_or(NetworkError::StunNoServers {
            servers: self.servers.clone(),
        }))
    }

    /// UDP hole-punching: returns a bound socket plus the external address.
    pub async fn bind(&self) -> Result<(TokioUdpSocket, StunResponse), NetworkError> {
        let socket = TokioUdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::UdpBind { source: e })?;
        let external = self.query_external_address().await?;
        Ok((socket, external))
    }

    async fn send_binding_request(
        &self,
        server: &str,
        transport: StunTransport,
    ) -> Result<StunResponse, NetworkError> {
        let request = build_binding_request();
        let mut last_error = None;

        for attempt in 0..self.config.retries {
            let result = match transport {
                StunTransport::Udp => self.send_udp(server, &request).await,
                StunTransport::Tcp => self.send_tcp(server, &request).await,
            };
            match result {
                Ok(r) => return Ok(r),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.retries - 1 {
                        tokio::time::sleep(Duration::from_millis(100 * (attempt as u64 + 1))).await;
                    }
                }
            }
        }
        Err(last_error.unwrap_or(NetworkError::StunTimeout {
            server: server.to_string(),
        }))
    }

    async fn send_udp(&self, server: &str, request: &[u8]) -> Result<StunResponse, NetworkError> {
        let addr = resolve_stun_addr(server)?;
        let socket = TokioUdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::UdpBind { source: e })?;
        socket
            .send_to(request, addr)
            .await
            .map_err(|e| NetworkError::UdpSend { source: e })?;
        let mut buf = vec![0u8; 1024];
        let (len, _) = timeout(self.config.timeout, socket.recv_from(&mut buf))
            .await
            .map_err(|_| NetworkError::StunTimeout {
                server: server.to_string(),
            })?
            .map_err(|e| NetworkError::UdpRecv { source: e })?;
        parse_response(&buf[..len], server)
    }

    async fn send_tcp(&self, server: &str, request: &[u8]) -> Result<StunResponse, NetworkError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let addr = resolve_stun_addr(server)?;
        let mut stream = timeout(self.config.timeout, tokio::net::TcpStream::connect(addr))
            .await
            .map_err(|_| NetworkError::StunTimeout {
                server: server.to_string(),
            })?
            .map_err(|e| NetworkError::StunTcp {
                server: server.to_string(),
                source: e,
            })?;
        stream
            .write_all(request)
            .await
            .map_err(|e| NetworkError::StunTcp {
                server: server.to_string(),
                source: e,
            })?;
        let mut buf = vec![0u8; 1024];
        let len = timeout(self.config.timeout, stream.read(&mut buf))
            .await
            .map_err(|_| NetworkError::StunTimeout {
                server: server.to_string(),
            })?
            .map_err(|e| NetworkError::StunTcp {
                server: server.to_string(),
                source: e,
            })?;
        parse_response(&buf[..len], server)
    }
}

fn resolve_stun_addr(server: &str) -> Result<SocketAddr, NetworkError> {
    server
        .to_socket_addrs()
        .map_err(|e| NetworkError::StunDns {
            server: server.to_string(),
            source: e,
        })?
        .find(|a| a.is_ipv4())
        .ok_or_else(|| NetworkError::StunDns {
            server: server.to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "No IPv4 address"),
        })
}

/// Builds a STUN Binding Request (RFC 5389 section 6).
fn build_binding_request() -> Vec<u8> {
    let mut msg = vec![0u8; HEADER_SIZE];
    msg[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    msg[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    let tid = make_transaction_id();
    msg[8..20].copy_from_slice(&tid);
    msg
}

fn make_transaction_id() -> [u8; 12] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut id = [0u8; 12];
    id[0..8].copy_from_slice(&(nanos as u64).to_be_bytes());
    id[8..12].copy_from_slice(&(nanos as u32).to_be_bytes());
    id
}

/// Parses a STUN Binding Response.
fn parse_response(data: &[u8], server: &str) -> Result<StunResponse, NetworkError> {
    if data.len() < HEADER_SIZE {
        return Err(NetworkError::StunParse {
            message: "Response too short for STUN header".into(),
        });
    }
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != BINDING_RESPONSE {
        return Err(NetworkError::StunParse {
            message: format!("Expected Binding Response (0x0101), got 0x{msg_type:04X}"),
        });
    }
    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let cookie = u32::from_be_bytes(data[4..8].try_into().unwrap());
    if cookie != MAGIC_COOKIE {
        return Err(NetworkError::StunParse {
            message: "Invalid magic cookie".into(),
        });
    }
    let tid: &[u8; 12] = data[8..20].try_into().unwrap();
    let attrs = &data[HEADER_SIZE..std::cmp::min(HEADER_SIZE + msg_len, data.len())];

    let mut mapped = None;
    let mut xor_mapped = None;
    let mut changed = None;
    let mut pos = 0;

    while pos + ATTR_HDR <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[pos], attrs[pos + 1]]);
        let attr_len = u16::from_be_bytes([attrs[pos + 2], attrs[pos + 3]]) as usize;
        if pos + ATTR_HDR + attr_len > attrs.len() {
            break;
        }
        let d = &attrs[pos + ATTR_HDR..pos + ATTR_HDR + attr_len];
        match attr_type {
            ATTR_MAPPED => {
                if let Some(a) = parse_mapped(d) {
                    mapped = Some(a);
                }
            }
            ATTR_XOR_MAPPED => {
                if let Some(a) = parse_xor_mapped(d, tid) {
                    xor_mapped = Some(a);
                }
            }
            ATTR_CHANGED => {
                if let Some(a) = parse_mapped(d) {
                    changed = Some(a);
                }
            }
            _ => {}
        }
        pos += ATTR_HDR + attr_len;
        pos = (pos + 3) & !3;
    }

    let external_addr = xor_mapped
        .or(mapped)
        .ok_or_else(|| NetworkError::StunParse {
            message: "No MAPPED-ADDRESS or XOR-MAPPED-ADDRESS in response".into(),
        })?;

    Ok(StunResponse {
        external_addr,
        mapped_addr: mapped.unwrap_or(external_addr),
        changed_addr: changed,
        server: server.to_string(),
    })
}

/// Parses MAPPED-ADDRESS: [0, family(1), port(2), addr(4/16)].
fn parse_mapped(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 4 {
        return None;
    }
    let port = u16::from_be_bytes([data[2], data[3]]);
    match data[1] {
        0x01 if data.len() >= 8 => {
            let ip = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Some(SocketAddr::new(ip.into(), port))
        }
        0x02 if data.len() >= 20 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&data[4..20]);
            Some(SocketAddr::new(
                std::net::Ipv6Addr::from(octets).into(),
                port,
            ))
        }
        _ => None,
    }
}

/// Parses XOR-MAPPED-ADDRESS (RFC 5389 section 15.2).
fn parse_xor_mapped(data: &[u8], tid: &[u8; 12]) -> Option<SocketAddr> {
    if data.len() < 4 {
        return None;
    }
    let port = u16::from_be_bytes([data[2], data[3]]) ^ (MAGIC_COOKIE >> 16) as u16;
    match data[1] {
        0x01 if data.len() >= 8 => {
            let raw = u32::from_be_bytes(data[4..8].try_into().unwrap()) ^ MAGIC_COOKIE;
            Some(SocketAddr::new(
                Ipv4Addr::from(raw.to_be_bytes()).into(),
                port,
            ))
        }
        0x02 if data.len() >= 20 => {
            let mut a = [0u8; 16];
            a.copy_from_slice(&data[4..20]);
            for (i, slot) in a.iter_mut().take(4).enumerate() {
                *slot ^= (MAGIC_COOKIE >> (24 - 8 * i)) as u8;
            }
            for i in 0..12 {
                a[4 + i] ^= tid[i];
            }
            Some(SocketAddr::new(std::net::Ipv6Addr::from(a).into(), port))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_request_format() {
        let req = build_binding_request();
        assert_eq!(req.len(), HEADER_SIZE);
        assert_eq!(&req[0..2], &[0x00, 0x01]);
        assert_eq!(&req[2..4], &[0x00, 0x00]);
        assert_eq!(
            u32::from_be_bytes(req[4..8].try_into().unwrap()),
            MAGIC_COOKIE
        );
    }

    #[test]
    fn parse_mapped_ipv4() {
        let data = [0x00, 0x01, 0x00, 0x50, 0xC0, 0xA8, 0x01, 0x01];
        assert_eq!(parse_mapped(&data), Some("192.168.1.1:80".parse().unwrap()));
    }

    #[test]
    fn parse_mapped_short() {
        assert!(parse_mapped(&[0x00, 0x01]).is_none());
    }

    #[test]
    fn parse_xor_mapped_ipv4() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let port: u16 = 12345;
        let xport = port ^ (MAGIC_COOKIE >> 16) as u16;
        let xip = u32::from_be_bytes(ip.octets()) ^ MAGIC_COOKIE;
        let mut data = vec![0x00, 0x01];
        data.extend_from_slice(&xport.to_be_bytes());
        data.extend_from_slice(&xip.to_be_bytes());
        assert_eq!(
            parse_xor_mapped(&data, &[0u8; 12]),
            Some("10.0.0.1:12345".parse().unwrap())
        );
    }

    #[test]
    fn client_defaults() {
        let c = StunClient::new();
        assert!(c.servers.contains(&"stun.l.google.com:19302".into()));
        assert_eq!(c.config.transport, StunTransport::Udp);
    }

    #[test]
    fn client_custom_servers() {
        let s = vec!["my-stun.example.com:3478".into()];
        let c = StunClient::with_servers(s.clone());
        assert_eq!(c.servers, s);
    }

    #[test]
    fn response_serializes() {
        let r = StunResponse {
            external_addr: "1.2.3.4:5678".parse().unwrap(),
            mapped_addr: "1.2.3.4:5678".parse().unwrap(),
            changed_addr: None,
            server: "stun.l.google.com:19302".into(),
        };
        assert!(serde_json::to_string(&r).unwrap().contains("external_addr"));
    }

    #[test]
    fn response_too_short() {
        assert!(parse_response(&[0u8; 5], "test").is_err());
    }

    #[test]
    fn response_wrong_type() {
        let mut data = [0u8; HEADER_SIZE];
        data[1] = 0x02;
        assert!(parse_response(&data, "test").is_err());
    }
}
