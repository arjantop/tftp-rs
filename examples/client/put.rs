extern crate tftp;

use std::old_io::{File, BufferedReader};
use std::old_io::net::ip::{SocketAddr, Ipv4Addr};
use std::env;

use tftp::client::Client;
use tftp::packet::Mode;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} PATH", args[0]);
        return
    }
    let file_path = args[1].clone();
    let file = File::open(&Path::new(&file_path[..]));
    let mut reader = BufferedReader::new(file);
    let result = Client::new(SocketAddr{
        ip: Ipv4Addr(127, 0, 0, 1),
        port: 69
    }).and_then(|mut client| {
        client.put(&Path::new("testfile"), Mode::Octet, &mut reader)
    });
    if result.is_err() {
        println!("error = {}", result.err().unwrap());
        env::set_exit_status(1);
    }
}
