// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_is_port_available_valid_address() {
    // Port 0 is special: the OS will always assign an ephemeral port on bind.
    let result: Result<bool> = is_port_available("127.0.0.1", 0);
    assert!(result.is_ok());
    assert_eq!(result.ok(), Some(true));
}

#[test]
fn test_is_port_available_invalid_address() {
    // An invalid address should return an error immediately.
    let result: Result<bool> = is_port_available("invalid_address", 8080);
    assert!(result.is_err());
}

#[test]
fn test_find_available_port_invalid_range() {
    let result: Result<u16> = find_available_port("127.0.0.1", 9000, 8000);
    assert!(result.is_err());
}

#[test]
fn test_resolve_http_port_uses_alternative() -> Result<()> {
    // Hold a listener on an OS-assigned port so it is occupied.
    let listener: TcpListener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| ::anyhow::anyhow!("failed to bind ephemeral port for test: {e}"))?;
    let occupied_port: u16 = listener
        .local_addr()
        .map_err(|e| ::anyhow::anyhow!("failed to get local address for test: {e}"))?
        .port();

    // Resolve should find a different port since the configured one is in use.
    let resolved_port: u16 = resolve_http_port("127.0.0.1", occupied_port)?;
    assert_ne!(resolved_port, occupied_port);

    Ok(())
}
