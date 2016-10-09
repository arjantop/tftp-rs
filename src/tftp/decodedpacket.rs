use std::mem;
use std::ops::Deref;

use packet::{RawPacket, DecodePacket};

pub struct DecodedPacket<P: Sized> {
    raw: RawPacket,
    packet: P,
}

impl<P: DecodePacket<'static>> DecodedPacket<P> {
    pub fn decode(raw: RawPacket) -> Option<DecodedPacket<P>> {
        let mut p = DecodedPacket {
            raw: raw,
            packet: unsafe { mem::uninitialized() },
        };
        P::decode(unsafe { extend_buf_lifetime(&p.raw.packet_buf()) }).map(|packet| {
            p.packet = packet;
            p
        })
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.raw.get_buffer()
    }
}

impl<P: DecodePacket<'static>> Deref for DecodedPacket<P> {
    type Target = P;

    fn deref(&self) -> &P {
        &self.packet
    }
}

unsafe fn extend_buf_lifetime<'a>(r: &'a [u8]) -> &'static [u8] {
    mem::transmute(r)
}
