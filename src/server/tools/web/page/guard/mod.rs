//! URL validation for agent-supplied addresses.
//!
//! `read_webpage` takes a URL chosen by a model, from text the model may have
//! read on another page. This module decides what is allowed to be fetched.
//!
//! ## What this can and cannot guarantee
//!
//! Literal-IP URLs are checked here and rejected when they point at private,
//! loopback, link-local or otherwise reserved space — including the cloud
//! metadata address.
//!
//! Hostnames are a different matter. In VPN mode the request is handed to a
//! proxy inside the tunnel, and DNS resolution happens *there*, not in this
//! process — which is what stops the query leaking, but also means this
//! process never sees the resolved address and cannot pin it. Resolving here
//! to check would both leak the lookup and be defeated by DNS rebinding, since
//! the proxy resolves independently afterwards.
//!
//! So the actual boundary against name-based SSRF is the tunnel: the proxy's
//! network is the one a hostname resolves on, and it is not the server's
//! private network. This module blocks the literal-address path and the
//! non-HTTP schemes; the network topology blocks the rest.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

#[cfg(test)]
mod tests;

/// Why a URL may not be fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlRejection {
    NotAbsolute,
    UnsupportedScheme(String),
    EmbeddedCredentials,
    PrivateAddress(String),
    NoHost,
}

impl std::fmt::Display for UrlRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlRejection::NotAbsolute => {
                write!(f, "not an absolute URL — include http:// or https://")
            }
            UrlRejection::UnsupportedScheme(s) => {
                write!(
                    f,
                    "unsupported scheme '{s}' — only http and https are fetched"
                )
            }
            UrlRejection::EmbeddedCredentials => {
                write!(f, "URLs containing credentials are not fetched")
            }
            UrlRejection::PrivateAddress(a) => {
                write!(f, "'{a}' is a private or reserved address")
            }
            UrlRejection::NoHost => write!(f, "URL has no host"),
        }
    }
}

/// Validate an agent-supplied URL, returning the normalized form.
///
/// The fragment is stripped (it never reaches the server) and the query is
/// preserved (it often selects the content).
pub fn validate(raw: &str) -> Result<Url, UrlRejection> {
    let mut url = Url::parse(raw.trim()).map_err(|_| UrlRejection::NotAbsolute)?;

    match url.scheme() {
        "http" | "https" => {}
        other => return Err(UrlRejection::UnsupportedScheme(other.to_string())),
    }

    // `user:pass@host` would send credentials the user never granted, and is
    // also the classic way to disguise the real host from a human reader.
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlRejection::EmbeddedCredentials);
    }

    match url.host() {
        None => return Err(UrlRejection::NoHost),
        Some(Host::Ipv4(ip)) => {
            if !is_public_v4(ip) {
                return Err(UrlRejection::PrivateAddress(ip.to_string()));
            }
        }
        Some(Host::Ipv6(ip)) => {
            if !is_public_v6(ip) {
                return Err(UrlRejection::PrivateAddress(ip.to_string()));
            }
        }
        Some(Host::Domain(_)) => {}
    }

    url.set_fragment(None);
    Ok(url)
}

/// Whether a redirect target is acceptable.
///
/// Applied to every hop: a public URL redirecting to `127.0.0.1` is the
/// standard way to turn an allowed fetch into an internal one.
pub fn validate_hop(url: &Url) -> Result<(), UrlRejection> {
    validate(url.as_str()).map(|_| ())
}

/// Whether an IPv4 address is routable public space.
fn is_public_v4(ip: Ipv4Addr) -> bool {
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()      // 169.254/16, including 169.254.169.254
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
        // 100.64/10 carrier-grade NAT
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 64)
        // 192.0.0/24 IETF protocol assignments
        || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
        // 198.18/15 benchmarking
        || (ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18)
        // 240/4 reserved
        || ip.octets()[0] >= 240)
}

/// Whether an IPv6 address is routable public space.
fn is_public_v6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    let seg = ip.segments();
    // fc00::/7 unique local
    if (seg[0] & 0xfe00) == 0xfc00 {
        return false;
    }
    // fe80::/10 link local
    if (seg[0] & 0xffc0) == 0xfe80 {
        return false;
    }
    // 2001:db8::/32 documentation
    if seg[0] == 0x2001 && seg[1] == 0x0db8 {
        return false;
    }
    // ::ffff:0:0/96 IPv4-mapped — check the embedded address instead
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    // ::a.b.c.d IPv4-compatible (deprecated, still parsed)
    if seg[..6] == [0, 0, 0, 0, 0, 0] {
        if let IpAddr::V4(v4) = IpAddr::from(ip).to_canonical() {
            return is_public_v4(v4);
        }
    }
    true
}
