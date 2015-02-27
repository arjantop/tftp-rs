extern crate tftp;

use std::old_io::{File, Write, Truncate, BufferedWriter};
use std::old_io::net::ip::{SocketAddr, Ipv4Addr};
use std::os;

use tftp::client::{Client, ClientError};
use tftp::packet::Mode;

fn main() {
    let args = os::args();
    if args.len() != 2 {
        println!("Usage: {} PATH", args.get(0).unwrap());
        return
    }
    let file_path = args[1].clone();
    let file = File::open_mode(&Path::new("/tmp/result"), Truncate, Write);
    let mut writer = BufferedWriter::new(file);
    let result = Client::new(SocketAddr{
        ip: Ipv4Addr(127, 0, 0, 1),
        port: 69
    }).map_err(ClientError::from_io).and_then(|mut client| {
        client.get(&Path::new(file_path.as_slice()), Mode::Octet, &mut writer)
    });
    if result.is_err() {
        // FIXME
        println!("error");
        //println!("error = {}", result.err().unwrap());
        os::set_exit_status(1);
    }
}
