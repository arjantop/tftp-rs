#![feature(ip_addr)]
extern crate tftp;

use std::io::BufWriter;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::process::exit;
use std::env;

use tftp::client::Client;
use tftp::packet::Mode;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} PATH", args.get(0).unwrap());
        return
    }
    let file_path = args[1].clone();
    let mut file_options = OpenOptions::new();
    file_options.truncate(true).create(true).write(true);
    let file = match file_options.open(Path::new("/tmp/result")) {
        Ok(f) => f,
        Err(_) => {
            exit(1);
        },
    };
    let mut writer = BufWriter::new(file);
    let result = Client::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 69)).and_then(|mut client| {
        client.get(&Path::new(&file_path), Mode::Octet, &mut writer)
    });
    if result.is_err() {
        // FIXME
        println!("error");
        //println!("error = {}", result.err().unwrap());
        exit(1);
    }
}
