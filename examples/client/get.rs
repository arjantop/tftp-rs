extern crate tftp;

use std::io::BufWriter;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::env;
use std::net::{SocketAddr, IpAddr};

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
            env::set_exit_status(1);
            return
        },
    };
    let mut writer = BufWriter::new(file);
    let result = Client::new(SocketAddr::new(IpAddr::new_v4(127, 0, 0, 1), 69)).and_then(|mut client| {
        client.get(&Path::new(file_path.as_slice()), Mode::Octet, &mut writer)
    });
    if result.is_err() {
        // FIXME
        println!("error");
        //println!("error = {}", result.err().unwrap());
        env::set_exit_status(1);
    }
}
