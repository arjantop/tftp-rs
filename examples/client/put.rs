extern crate tftp;

use std::io::BufReader;
use std::fs::File;
use std::path::Path;
use std::env;
use std::process::exit;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use tftp::client::Client;
use tftp::packet::Mode;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} PATH", args.get(0).unwrap());
        return
    }
    let file_path = args[1].clone();
    let file = match File::open(Path::new("/tmp/result")) {
        Ok(f) => f,
        Err(_) => {
            exit(1);
        },
    };
    let mut reader = BufReader::new(file);
    let result = Client::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 69)).and_then(|mut client| {
        client.put(&Path::new(&file_path), Mode::Octet, &mut reader)
    });
    if result.is_err() {
        // FIXME
        println!("error");
        //println!("error = {}", result.err().unwrap());
        exit(1);
    }
}
