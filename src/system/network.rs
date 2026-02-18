//! Network data collection: aggregate and per-interface statistics,
//! active TCP connections.

use std::collections::HashMap;

use sysinfo::Networks;

/// Aggregate network traffic snapshot.
#[derive(Clone, Debug, Default)]
pub struct NetSnapshot {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// Bytes/sec received.
    pub rx_speed: f64,
    /// Bytes/sec transmitted.
    pub tx_speed: f64,
}

/// Per-interface network statistics.
#[derive(Clone, Debug)]
pub struct InterfaceStats {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_speed: f64,
    pub tx_speed: f64,
}

/// A single active TCP connection (from /proc/net/tcp).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TcpConnection {
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub uid: u32,
}

pub fn collect_aggregate(
    networks: &Networks,
    prev_rx: &mut u64,
    prev_tx: &mut u64,
    dt_secs: f64,
) -> NetSnapshot {
    let (rx, tx) = networks
        .iter()
        .fold((0u64, 0u64), |(r, t), (_name, data)| {
            (r + data.total_received(), t + data.total_transmitted())
        });

    let rx_delta = rx.saturating_sub(*prev_rx);
    let tx_delta = tx.saturating_sub(*prev_tx);
    *prev_rx = rx;
    *prev_tx = tx;

    let dt = if dt_secs > 0.0 { dt_secs } else { 1.0 };

    NetSnapshot {
        rx_bytes: rx,
        tx_bytes: tx,
        rx_speed: rx_delta as f64 / dt,
        tx_speed: tx_delta as f64 / dt,
    }
}

pub fn collect_per_interface(
    networks: &Networks,
    prev_rx: &mut HashMap<String, u64>,
    prev_tx: &mut HashMap<String, u64>,
    dt_secs: f64,
) -> Vec<InterfaceStats> {
    let dt = if dt_secs > 0.0 { dt_secs } else { 1.0 };

    networks
        .iter()
        .map(|(name, data)| {
            let rx = data.total_received();
            let tx = data.total_transmitted();
            let prev_r = prev_rx.get(name.as_str()).copied().unwrap_or(0);
            let prev_t = prev_tx.get(name.as_str()).copied().unwrap_or(0);

            let rx_speed = rx.saturating_sub(prev_r) as f64 / dt;
            let tx_speed = tx.saturating_sub(prev_t) as f64 / dt;

            prev_rx.insert(name.to_string(), rx);
            prev_tx.insert(name.to_string(), tx);

            InterfaceStats {
                name: name.to_string(),
                rx_bytes: rx,
                tx_bytes: tx,
                rx_speed,
                tx_speed,
            }
        })
        .collect()
}

/// Read active TCP connections from /proc/net/tcp.
pub fn active_tcp_connections() -> Vec<TcpConnection> {
    let mut conns = Vec::new();

    if let Ok(contents) = std::fs::read_to_string("/proc/net/tcp") {
        for line in contents.lines().skip(1) {
            if let Some(conn) = parse_tcp_line(line) {
                conns.push(conn);
            }
        }
    }
    // Also try IPv6
    if let Ok(contents) = std::fs::read_to_string("/proc/net/tcp6") {
        for line in contents.lines().skip(1) {
            if let Some(conn) = parse_tcp6_line(line) {
                conns.push(conn);
            }
        }
    }

    conns
}

fn parse_tcp_line(line: &str) -> Option<TcpConnection> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 10 {
        return None;
    }

    let local = parse_addr_port(fields[1])?;
    let remote = parse_addr_port(fields[2])?;
    let state_code = u8::from_str_radix(fields[3], 16).unwrap_or(0);
    let uid = fields[7].parse::<u32>().unwrap_or(0);

    Some(TcpConnection {
        local_addr: local.0,
        local_port: local.1,
        remote_addr: remote.0,
        remote_port: remote.1,
        state: tcp_state_name(state_code),
        uid,
    })
}

fn parse_tcp6_line(line: &str) -> Option<TcpConnection> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 10 {
        return None;
    }

    let local = parse_addr6_port(fields[1])?;
    let remote = parse_addr6_port(fields[2])?;
    let state_code = u8::from_str_radix(fields[3], 16).unwrap_or(0);
    let uid = fields[7].parse::<u32>().unwrap_or(0);

    Some(TcpConnection {
        local_addr: local.0,
        local_port: local.1,
        remote_addr: remote.0,
        remote_port: remote.1,
        state: tcp_state_name(state_code),
        uid,
    })
}

/// Parse "0100007F:1F90" → ("127.0.0.1", 8080)
fn parse_addr_port(s: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let addr_hex = u32::from_str_radix(parts[0], 16).unwrap_or(0);
    let port = u16::from_str_radix(parts[1], 16).unwrap_or(0);
    let addr = format!(
        "{}.{}.{}.{}",
        addr_hex & 0xFF,
        (addr_hex >> 8) & 0xFF,
        (addr_hex >> 16) & 0xFF,
        (addr_hex >> 24) & 0xFF,
    );
    Some((addr, port))
}

/// Parse IPv6 address:port hex string.
fn parse_addr6_port(s: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let port = u16::from_str_radix(parts[1], 16).unwrap_or(0);
    // Simplified: just show "IPv6" or try to detect mapped v4
    let hex = parts[0];
    if hex.len() == 32 && hex.starts_with("0000000000000000FFFF0000") {
        // IPv4-mapped IPv6
        let v4_hex = &hex[24..];
        if let Ok(addr_hex) = u32::from_str_radix(v4_hex, 16) {
            let addr = format!(
                "{}.{}.{}.{}",
                addr_hex & 0xFF,
                (addr_hex >> 8) & 0xFF,
                (addr_hex >> 16) & 0xFF,
                (addr_hex >> 24) & 0xFF,
            );
            return Some((addr, port));
        }
    }
    Some(("[::]".to_string(), port))
}

fn tcp_state_name(code: u8) -> String {
    match code {
        0x01 => "ESTABLISHED".to_string(),
        0x02 => "SYN_SENT".to_string(),
        0x03 => "SYN_RECV".to_string(),
        0x04 => "FIN_WAIT1".to_string(),
        0x05 => "FIN_WAIT2".to_string(),
        0x06 => "TIME_WAIT".to_string(),
        0x07 => "CLOSE".to_string(),
        0x08 => "CLOSE_WAIT".to_string(),
        0x09 => "LAST_ACK".to_string(),
        0x0A => "LISTEN".to_string(),
        0x0B => "CLOSING".to_string(),
        _ => format!("0x{:02X}", code),
    }
}
