use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use clap::Parser;
use osp_server_sdk::OSProtocolNode;
use url::Url;
use osp_protocol::OSPUrl;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IPv4 address to bind to
    #[arg(short, long)]
    bind: String,

    /// TCP port to bind to
    #[arg(short, long, default_value_t = 42069)]
    port: u16,

    /// RSA Private Key for decrypting DNS challenges
    #[arg(long)]
    private_key: String,

    /// Used to identify myself during the handshake
    #[arg(long)]
    hostname: String,

    /// Servers to open outbound connections to
    #[arg(long)]
    push_to: Vec<String>
}

fn main() {
    let args = Args::parse();
    let addr = SocketAddrV4::new(args.bind.parse().expect("Invalid bind address"), args.port);
    let node = Arc::new(Mutex::new(OSProtocolNode::builder().bind_to(SocketAddr::from(addr)).build()));

    let n = Arc::clone(&node);
    std::thread::spawn(move || {
        n.lock().unwrap().listen()
    });

    for uri in args.push_to {
        let osp_url = OSPUrl::from(Url::parse(uri.as_str()).unwrap());
        let n = Arc::clone(&node);
        std::thread::spawn(move || {
            n.lock().unwrap().test_outbound(osp_url)
        });
    }
}
