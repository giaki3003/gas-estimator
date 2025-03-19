use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

/// Spawns an Anvil process on a free port and returns the process handle and the RPC URL.
///
/// # Panics
///
/// Panics if it fails to bind to a free port or spawn Anvil.
pub fn spawn_anvil() -> (Child, String) {
    // Bind to a free port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Could not bind to port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    // Construct the RPC URL for Anvil
    let rpc_url = format!("http://127.0.0.1:{}", port);

    // Spawn the Anvil process
    let child = Command::new("anvil")
        .arg("-p")
        .arg(port.to_string())
        .arg("--hardfork")
        .arg("prague") // Necessary for eip7702 tests to succeed
        // Optionally, redirect stdout and stderr if you need to debug
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn anvil");

    // Wait a moment to ensure Anvil is up and running
    sleep(Duration::from_secs(1));

    (child, rpc_url)
}