//! UPnP port forwarding client.
//!
//! Discovers UPnP-enabled routers via SSDP multicast and manages port mappings
//! via SOAP control requests. Used to establish inbound connectivity for
//! peer-to-peer mesh networking when the daemon sits behind a NAT gateway.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::time::{timeout, Duration};

use super::NetworkError;

const SSDP_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;

const IGD_SERVICE_TYPE: &str = "urn:schemas-upnp-org:device:InternetGatewayDevice:1";
const WAN_SERVICE_TYPE: &str =
    "urn:schemas-upnp-org:service:WANIPConnection:1";

const MSEARCH_TEMPLATE: &str = "M-SEARCH * HTTP/1.1\r\n\
    HOST: 239.255.255.250:1900\r\n\
    MAN: \"ssdp:discover\"\r\n\
    MX: 3\r\n\
    ST: {st}\r\n\
    \r\n";

const SSDP_BUFFER_SIZE: usize = 4096;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const SOAP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Tcp => "TCP",
            Protocol::Udp => "UDP",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TCP" => Some(Protocol::Tcp),
            "UDP" => Some(Protocol::Udp),
            _ => None,
        }
    }
}

/// A UPnP port mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// External (WAN) port to expose.
    pub external_port: u16,
    /// Internal (LAN) port to forward traffic to.
    pub internal_port: u16,
    /// Transport protocol (TCP or UDP).
    pub protocol: Protocol,
    /// Human-readable description of this mapping.
    pub description: String,
    /// Lease duration in seconds. 0 means permanent (router-dependent).
    pub lease_duration: u32,
}

/// Information about a discovered UPnP gateway device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayInfo {
    /// The control URL for the WAN IP connection service.
    pub control_url: String,
    /// The external (public) IP address reported by the gateway.
    pub external_ip: Option<String>,
    /// The manufacturer of the gateway device.
    pub manufacturer: Option<String>,
    /// The friendly name of the gateway device.
    pub friendly_name: Option<String>,
}

/// UPnP client for discovering gateways and managing port mappings.
#[derive(Debug, Clone)]
pub struct UPnPClient {
    /// Timeout for SSDP discovery.
    discovery_timeout: Duration,
    /// Timeout for SOAP requests.
    soap_timeout: Duration,
}

impl UPnPClient {
    /// Creates a new UPnP client with default timeouts.
    pub fn new() -> Self {
        Self {
            discovery_timeout: DISCOVERY_TIMEOUT,
            soap_timeout: SOAP_TIMEOUT,
        }
    }

    /// Creates a new UPnP client with custom timeouts.
    pub fn with_timeouts(discovery: Duration, soap: Duration) -> Self {
        Self {
            discovery_timeout: discovery,
            soap_timeout: soap,
        }
    }

    /// Discovers a UPnP gateway on the local network.
    ///
    /// Sends an SSDP M-SEARCH multicast, parses responses to locate the
    /// gateway's control URL, and fetches the external IP address.
    pub async fn discover(&self) -> Result<GatewayInfo, NetworkError> {
        let response = self.send_msearch().await?;
        let control_url = self.parse_control_url(&response)?;

        // Build a temporary GatewayInfo to query external IP.
        let temp_gw = GatewayInfo {
            control_url: control_url.clone(),
            external_ip: None,
            manufacturer: self.extract_header(&response, "Manufacturer"),
            friendly_name: self.extract_header(&response, "FriendlyName"),
        };
        let external_ip = self
            .get_external_ip(&temp_gw)
            .await
            .ok()
            .flatten();

        Ok(GatewayInfo {
            external_ip,
            ..temp_gw
        })
    }

    /// Adds a port mapping on the gateway.
    pub async fn add_port_mapping(
        &self,
        gateway: &GatewayInfo,
        mapping: PortMapping,
    ) -> Result<(), NetworkError> {
        let local_ip = self.get_local_ip()?;
        let soap_body = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:AddPortMapping xmlns:u="{WAN_SERVICE_TYPE}">
      <NewRemoteHost></NewRemoteHost>
      <NewExternalPort>{}</NewExternalPort>
      <NewProtocol>{}</NewProtocol>
      <NewInternalPort>{}</NewInternalPort>
      <NewInternalClient>{}</NewInternalClient>
      <NewEnabled>1</NewEnabled>
      <NewPortMappingDescription>{}</NewPortMappingDescription>
      <NewLeaseDuration>{}</NewLeaseDuration>
    </u:AddPortMapping>
  </s:Body>
</s:Envelope>"#,
            mapping.external_port,
            mapping.protocol.as_str(),
            mapping.internal_port,
            local_ip,
            xml_escape(&mapping.description),
            mapping.lease_duration,
        );

        let action = "AddPortMapping";
        self.send_soap_request(gateway, action, &soap_body)
            .await
            .map(|_| ())
    }

    /// Removes a previously created port mapping.
    pub async fn remove_port_mapping(
        &self,
        gateway: &GatewayInfo,
        external_port: u16,
        protocol: Protocol,
    ) -> Result<(), NetworkError> {
        let soap_body = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:DeletePortMapping xmlns:u="{WAN_SERVICE_TYPE}">
      <NewRemoteHost></NewRemoteHost>
      <NewExternalPort>{}</NewExternalPort>
      <NewProtocol>{}</NewProtocol>
    </u:DeletePortMapping>
  </s:Body>
</s:Envelope>"#,
            external_port,
            protocol.as_str(),
        );

        let action = "DeletePortMapping";
        self.send_soap_request(gateway, action, &soap_body)
            .await
            .map(|_| ())
    }

    /// Retrieves the external (public) IP address from the gateway.
    pub async fn get_external_ip(
        &self,
        gateway: &GatewayInfo,
    ) -> Result<Option<String>, NetworkError> {
        let soap_body = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:GetExternalIPAddress xmlns:u="{WAN_SERVICE_TYPE}">
    </u:GetExternalIPAddress>
  </s:Body>
</s:Envelope>"#
        );

        let response = self
            .send_soap_request(gateway, "GetExternalIPAddress", &soap_body)
            .await?;

        // Extract the IP from the SOAP response body.
        let ip = extract_tag(&response, "NewExternalIPAddress");
        Ok(ip)
    }

    // -- Private helpers --

    async fn send_msearch(&self) -> Result<String, NetworkError> {
        let msearch = MSEARCH_TEMPLATE.replace("{st}", IGD_SERVICE_TYPE);
        let msearch_bytes = msearch.as_bytes();

        let socket = TokioUdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::UdpBind {
                source: e,
            })?;

        socket
            .set_broadcast(true)
            .map_err(|e| NetworkError::UdpBind { source: e })?;

        let dest = SocketAddr::new(SSDP_MULTICAST_ADDR.into(), SSDP_PORT);

        socket
            .send_to(msearch_bytes, dest)
            .await
            .map_err(|e| NetworkError::UdpSend {
                source: e,
            })?;

        let mut buf = vec![0u8; SSDP_BUFFER_SIZE];

        let (len, _addr) = timeout(self.discovery_timeout, socket.recv_from(&mut buf))
            .await
            .map_err(|_| NetworkError::DiscoveryTimeout)?
            .map_err(|e| NetworkError::UdpRecv { source: e })?;

        let response = String::from_utf8_lossy(&buf[..len]).to_string();
        Ok(response)
    }

    fn parse_control_url(&self, ssdp_response: &str) -> Result<String, NetworkError> {
        let location = ssdp_response
            .lines()
            .find_map(|line| {
                let line = line.trim();
                line.to_lowercase()
                    .starts_with("location:")
                    .then(|| line.split_once(':'))
                    .flatten()
                    .map(|s| s.1.trim().to_string())
            })
            .ok_or_else(|| NetworkError::UpnpDiscovery {
                message: "No Location header in SSDP response".into(),
            })?;

        let xml = self.fetch_device_description(&location)?;
        extract_control_url(&xml)
    }

    fn fetch_device_description(&self, location: &str) -> Result<String, NetworkError> {
        let response = reqwest::blocking::get(location)
            .map_err(|e| NetworkError::UpnpDiscovery {
                message: format!("Failed to fetch device description: {e}"),
            })?;

        response.text().map_err(|e| NetworkError::UpnpDiscovery {
            message: format!("Failed to read device description: {e}"),
        })
    }

    /// Sends a SOAP request to the gateway and returns the response body.
    async fn send_soap_request(
        &self,
        gateway: &GatewayInfo,
        action: &str,
        body: &str,
    ) -> Result<String, NetworkError> {
        let url = format!("http://{}{}", self.gateway_host(gateway)?, gateway.control_url);
        let action_header = format!("{WAN_SERVICE_TYPE}#{action}");

        let client = reqwest::Client::builder()
            .timeout(self.soap_timeout)
            .build()
            .map_err(|e| NetworkError::UpnpSoap {
                action: action.into(),
                message: format!("Failed to build HTTP client: {e}"),
            })?;

        let response = client
            .post(&url)
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header("SOAPAction", &action_header)
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| NetworkError::UpnpSoap {
                action: action.into(),
                message: format!("SOAP request failed: {e}"),
            })?;

        response.text().await.map_err(|e| NetworkError::UpnpSoap {
            action: action.into(),
            message: format!("Failed to read SOAP response: {e}"),
        })
    }

    fn gateway_host(&self, gateway: &GatewayInfo) -> Result<String, NetworkError> {
        Ok(format!(
            "{}",
            gateway
                .control_url
                .split('/')
                .nth(2)
                .unwrap_or("localhost")
        ))
    }

    fn get_local_ip(&self) -> Result<String, NetworkError> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| NetworkError::UdpBind { source: e })?;
        socket.connect("8.8.8.8:80").map_err(|e| NetworkError::UdpBind { source: e })?;
        let local_addr = socket.local_addr().map_err(|e| NetworkError::UdpBind { source: e })?;
        Ok(match local_addr {
            SocketAddr::V4(v4) => v4.ip().to_string(),
            SocketAddr::V6(v6) => v6.ip().to_string(),
        })
    }

    fn extract_header(&self, response: &str, header: &str) -> Option<String> {
        response.lines().find_map(|line| {
            let line = line.trim();
            line.to_lowercase()
                .starts_with(&header.to_lowercase())
                .then(|| line.split_once(':'))
                .flatten()
                .map(|s| s.1.trim().to_string())
        })
    }
}

/// Extracts the WAN IP Connection control URL from device description XML.
fn extract_control_url(xml: &str) -> Result<String, NetworkError> {
    // Find the service with WANIPConnection type and extract its controlURL.
    let service_type_tag = WAN_SERVICE_TYPE;
    let mut found_service = false;
    let mut depth = 0u32;

    for line in xml.lines() {
        let trimmed = line.trim();

        if trimmed.contains(service_type_tag) {
            found_service = true;
        }

        if found_service && trimmed.starts_with("<controlURL>") {
            let start = trimmed.find('>').map(|i| i + 1).unwrap_or(0);
            let end = trimmed.find("</controlURL>").unwrap_or(trimmed.len());
            let url = trimmed[start..end].trim();
            if !url.is_empty() {
                return Ok(url.to_string());
            }
        }

        // Track nested elements after finding the service type.
        if found_service {
            depth += trimmed.matches('<').count() as u32;
            depth = depth.saturating_sub(trimmed.matches("</").count() as u32);
            if depth == 0 {
                break;
            }
        }
    }

    Err(NetworkError::UpnpDiscovery {
        message: "Could not find WANIPConnection controlURL in device description".into(),
    })
}

/// Extracts a tag's text content from an XML string.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    Some(xml[start..end].trim().to_string())
}

/// Escapes special XML characters.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_str_roundtrip() {
        assert_eq!(Protocol::Tcp.as_str(), "TCP");
        assert_eq!(Protocol::Udp.as_str(), "UDP");
        assert_eq!(Protocol::from_str("tcp"), Some(Protocol::Tcp));
        assert_eq!(Protocol::from_str("UDP"), Some(Protocol::Udp));
        assert_eq!(Protocol::from_str("invalid"), None);
    }

    #[test]
    fn xml_escape_handles_special_chars() {
        assert_eq!(
            xml_escape("a & b < c > d \"e\" f'g"),
            "a &amp; b &lt; c &gt; d &quot;e&quot; f&apos;g"
        );
    }

    #[test]
    fn extract_tag_basic() {
        let xml = "<foo>bar</foo>";
        assert_eq!(extract_tag(xml, "foo"), Some("bar".into()));
    }

    #[test]
    fn extract_tag_missing() {
        let xml = "<foo>bar</foo>";
        assert_eq!(extract_tag(xml, "baz"), None);
    }

    #[test]
    fn extract_control_url_from_xml() {
        let xml = r#"
        <root>
          <service>
            <serviceType>urn:schemas-upnp-org:service:WANIPConnection:1</serviceType>
            <controlURL>/ctl/C0</controlURL>
          </service>
        </root>"#;
        let url = extract_control_url(xml);
        assert_eq!(url.unwrap(), "/ctl/C0");
    }

    #[test]
    fn port_mapping_serializes() {
        let mapping = PortMapping {
            external_port: 8080,
            internal_port: 9000,
            protocol: Protocol::Tcp,
            description: "Test".into(),
            lease_duration: 3600,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        assert!(json.contains("\"external_port\":8080"));
    }

    #[test]
    fn client_construction() {
        let client = UPnPClient::new();
        assert_eq!(client.discovery_timeout, DISCOVERY_TIMEOUT);
        assert_eq!(client.soap_timeout, SOAP_TIMEOUT);
    }

    #[test]
    fn client_custom_timeouts() {
        let client = UPnPClient::with_timeouts(
            Duration::from_secs(10),
            Duration::from_secs(30),
        );
        assert_eq!(client.discovery_timeout, Duration::from_secs(10));
        assert_eq!(client.soap_timeout, Duration::from_secs(30));
    }
}
