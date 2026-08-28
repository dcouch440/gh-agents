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

use std::net::{Ipv4Addr, Ipv6Addr};

use thiserror::Error;
use url::{Host, Url};

#[cfg(test)]
mod tests;

/// Hostnames that always name the local machine or a well-known internal
/// service, regardless of what DNS says.
///
/// Matched exactly or as a parent suffix, after the trailing root dot is
/// stripped. These are the names that reach an internal address without ever
/// looking like a literal IP, so the address checks below never see them.
const BLOCKED_SUFFIXES: &[&str] = &[
    "localhost",
    "local",
    "internal",
    "localdomain",
    "metadata.google.internal",
];

/// Why a URL may not be fetched.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UrlRejection {
    #[error("not an absolute URL — include http:// or https://")]
    NotAbsolute,

    #[error("unsupported scheme '{0}' — only http and https are fetched")]
    UnsupportedScheme(String),

    #[error("URLs containing credentials are not fetched")]
    EmbeddedCredentials,

    #[error("'{0}' is a private or reserved address")]
    PrivateAddress(String),

    #[error("'{0}' names the local machine or an internal service")]
    InternalHostname(String),

    #[error("URL has no host")]
    NoHost,

    #[error("'{0}' could not be resolved")]
    Unresolvable(String),
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
        Some(Host::Domain(name)) => {
            if let Some(blocked) = blocked_hostname(name) {
                return Err(UrlRejection::InternalHostname(blocked));
            }
        }
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

/// Check the addresses a hostname actually resolves to.
///
/// Only meaningful when this process does its own DNS — see
/// [`crate::net::egress::resolves_locally`]. In that mode the name checks in
/// [`validate`] are not enough: `127.0.0.1.nip.io` and any attacker-controlled
/// name pointing at an internal address are ordinary domains that resolve
/// straight back to the host's own network.
///
/// A literal-address host has already been checked by [`validate`], so those
/// return early. This narrows but does not close DNS rebinding: the connection
/// resolves again afterwards, and nothing here pins the address.
pub async fn validate_addresses(url: &Url) -> Result<(), UrlRejection> {
    let name = match url.host() {
        Some(Host::Domain(d)) => d.to_string(),
        _ => return Ok(()),
    };
    let port = url.port_or_known_default().unwrap_or(80);

    let resolved = tokio::net::lookup_host((name.as_str(), port))
        .await
        .map_err(|_| UrlRejection::Unresolvable(name.clone()))?;

    let mut saw_any = false;
    for addr in resolved {
        saw_any = true;
        let public = match addr.ip() {
            std::net::IpAddr::V4(v4) => is_public_v4(v4),
            std::net::IpAddr::V6(v6) => is_public_v6(v6),
        };
        // Every address must pass: a name with one public and one loopback
        // record would otherwise be a coin flip at connect time.
        if !public {
            return Err(UrlRejection::PrivateAddress(format!(
                "{name} resolves to {}, which",
                addr.ip()
            )));
        }
    }
    if !saw_any {
        return Err(UrlRejection::Unresolvable(name));
    }
    Ok(())
}

/// Whether a hostname is one that always names something local.
///
/// Returns the offending name when it is. The comparison is case-insensitive
/// and ignores the DNS root dot, so `LocalHost.` is caught alongside
/// `localhost`. Suffix matching is on label boundaries: `notlocalhost` is a
/// perfectly ordinary name and is not blocked.
fn blocked_hostname(name: &str) -> Option<String> {
    let host = name.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    BLOCKED_SUFFIXES
        .iter()
        .any(|s| host == *s || host.ends_with(&format!(".{s}")))
        .then_some(host)
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
    // 64:ff9b::/96 NAT64 — the embedded v4 is the address actually reached.
    if seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2..6] == [0, 0, 0, 0] {
        return is_public_v4(embedded_v4(seg));
    }
    // ::a.b.c.d IPv4-compatible (deprecated, still parsed).
    //
    // Built from the segments directly: `to_canonical` only unwraps the
    // `::ffff:` *mapped* form, which the branch above has already handled, so
    // routing this through it would never match and the address would fall
    // through as public.
    if seg[..6] == [0, 0, 0, 0, 0, 0] && !ip.is_loopback() && !ip.is_unspecified() {
        return is_public_v4(embedded_v4(seg));
    }
    true
}

/// The IPv4 address embedded in the last two segments of an IPv6 address.
fn embedded_v4(seg: [u16; 8]) -> Ipv4Addr {
    Ipv4Addr::new(
        (seg[6] >> 8) as u8,
        (seg[6] & 0xff) as u8,
        (seg[7] >> 8) as u8,
        (seg[7] & 0xff) as u8,
    )
}
