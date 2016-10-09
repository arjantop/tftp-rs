// Copyright 2014 Arjan Topolovec
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//

//! High performance Trivial File Transfer Protocol (TFTP) protocol implementation.
//!
//! RFCs implemented:
//!
//! - RFC 1350 - TFTP Protocol (revision 2) (http://tools.ietf.org/html/rfc1350)

#![crate_name = "tftp"]
#![cfg_attr(test, feature(test))]

extern crate mio;
#[macro_use(quick_error)] extern crate quick_error;

pub mod packet;
pub mod netascii;
mod decodedpacket;

pub mod client;
