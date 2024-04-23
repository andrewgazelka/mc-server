use std::io::{Cursor, Write};

use anyhow::ensure;
use flate2::{bufread::ZlibEncoder, Compression};
use valence_protocol::{CompressionThreshold, Encode, Packet, VarInt};

mod util;

use crate::{events::ScratchBuffer, net::MAX_PACKET_SIZE, singleton::ring::Buf};

#[derive(Debug)]
pub struct PacketEncoder {
    threshold: CompressionThreshold,
    compression: Compression,
}

// todo:
// technically needs lifetimes to be write
// but ehhhh not doing this now we are referncing data which lives the duration of the program
// todo: bench if repr packed worth it (on old processors often slows down.
// Modern processors packed can actually be faster because cache locality)
#[allow(unused, reason = "this is used in linux")]
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct PacketWriteInfo {
    pub start_ptr: *const u8,
    pub len: u32,
}

impl PacketWriteInfo {
    #[allow(dead_code, reason = "nice for unit tests")]
    const unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.start_ptr, self.len as usize)
    }
}

unsafe impl Send for PacketWriteInfo {}
unsafe impl Sync for PacketWriteInfo {}

pub fn append_packet_without_compression<P>(
    pkt: &P,
    buf: &mut impl Buf,
) -> anyhow::Result<PacketWriteInfo>
where
    P: valence_protocol::Packet + Encode,
{
    let data_write_start = VarInt::MAX_SIZE as u64;
    let slice = buf.get_contiguous(MAX_PACKET_SIZE);

    let mut cursor = Cursor::new(slice);
    cursor.set_position(data_write_start);

    pkt.encode_with_id(&mut cursor)?;

    let data_len = cursor.position() as usize - data_write_start as usize;

    let packet_len_size = VarInt(data_len as i32).written_size();

    let packet_len = packet_len_size + data_len;
    ensure!(
        packet_len <= MAX_PACKET_SIZE,
        "packet exceeds maximum length"
    );

    let inner = cursor.into_inner();

    inner.copy_within(
        data_write_start as usize..data_write_start as usize + data_len,
        packet_len_size,
    );

    let mut cursor = Cursor::new(inner);
    VarInt(data_len as i32).encode(&mut cursor)?;

    let slice = cursor.into_inner();
    let entire_slice = &slice[..packet_len_size + data_len];

    let start_ptr = entire_slice.as_ptr();
    let len = entire_slice.len();

    buf.advance(len);

    Ok(PacketWriteInfo {
        start_ptr: start_ptr.cast(),
        len: len as u32,
    })
}

impl PacketEncoder {
    pub const fn new(threshold: CompressionThreshold, compression: Compression) -> Self {
        Self {
            threshold,
            compression,
        }
    }

    pub const fn compression_threshold(&self) -> CompressionThreshold {
        self.threshold
    }

    pub fn append_packet_with_compression<P>(
        &mut self,
        pkt: &P,
        buf: &mut impl Buf,
        scratch: &mut impl ScratchBuffer,
    ) -> anyhow::Result<PacketWriteInfo>
    where
        P: valence_protocol::Packet + Encode,
    {
        const DATA_LEN_0_SIZE: usize = 1;

        // + 1 because data len would be 0 if not compressed
        let data_write_start = (VarInt::MAX_SIZE + DATA_LEN_0_SIZE) as u64;
        let slice = buf.get_contiguous(MAX_PACKET_SIZE);

        let mut cursor = Cursor::new(&mut slice[..]);
        cursor.set_position(data_write_start);

        pkt.encode_with_id(&mut cursor)?;

        let end_data_position_exclusive = cursor.position();

        let data_len = end_data_position_exclusive - data_write_start;

        let threshold = u64::from(self.threshold.0.unsigned_abs());

        if data_len > threshold {
            let scratch = scratch.obtain();

            debug_assert!(scratch.is_empty());

            {
                let data_slice =
                    &mut slice[data_write_start as usize..end_data_position_exclusive as usize];
                let data_slice_cursor = Cursor::new(data_slice);
                let mut z = ZlibEncoder::new(data_slice_cursor, self.compression);
                // todo: is see if there is a more efficient way to do this. probs chunking would help or something
                // also this is a bit different than stdlib `default_read_to_end`.
                // However, it is needed because we are using a custom allocator
                util::read_to_end(&mut z, scratch)?;
            }

            let data_len = VarInt(data_len as u32 as i32);

            let packet_len = data_len.written_size() + scratch.len();
            let packet_len = VarInt(packet_len as u32 as i32);

            let mut write = Cursor::new(&mut slice[..]);
            packet_len.encode(&mut write)?;
            data_len.encode(&mut write)?;
            write.write_all(scratch)?;

            let len = write.position();

            let start_ptr = slice.as_ptr();
            buf.advance(len as usize);

            return Ok(PacketWriteInfo {
                start_ptr,
                len: len as u32,
            });
        }

        let data_len_0 = VarInt(0);
        let packet_len = VarInt(DATA_LEN_0_SIZE as i32 + data_len as u32 as i32); // packet_len.written_size();

        let mut cursor = Cursor::new(&mut slice[..]);
        packet_len.encode(&mut cursor)?;
        data_len_0.encode(&mut cursor)?;

        let pos = cursor.position();

        slice.copy_within(
            data_write_start as usize..end_data_position_exclusive as usize,
            pos as usize,
        );

        let len = pos as u32 + (end_data_position_exclusive - data_write_start) as u32;

        let start_ptr = slice.as_ptr();
        buf.advance(len as usize);

        Ok(PacketWriteInfo { start_ptr, len })
    }

    pub fn append_packet<P>(
        &mut self,
        pkt: &P,
        buf: &mut impl Buf,
        scratch: &mut impl ScratchBuffer,
    ) -> anyhow::Result<PacketWriteInfo>
    where
        P: Packet + Encode,
    {
        let has_compression = self.threshold.0 >= 0;

        if has_compression {
            self.append_packet_with_compression(pkt, buf, scratch)
        } else {
            append_packet_without_compression(pkt, buf)
        }
    }

    pub fn set_compression(&mut self, threshold: CompressionThreshold) {
        self.threshold = threshold;
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;
    use flate2::Compression;
    use valence_protocol::{
        packets::login, Bounded, CompressionThreshold, Encode, Packet,
        PacketEncoder as ValencePacketEncoder,
    };

    use crate::{
        events::Scratch,
        net::{encoder::PacketEncoder, MAX_PACKET_SIZE},
        singleton::ring::Ring,
    };

    fn compare_pkt<P: Packet + Encode>(packet: &P, compression: CompressionThreshold, msg: &str) {
        let mut large_ring = Ring::new(MAX_PACKET_SIZE * 2);

        let mut encoder = PacketEncoder::new(compression, Compression::new(4));

        let bump = Bump::new();
        let mut scratch = Scratch::from(&bump);
        let encoder_res = encoder
            .append_packet(packet, &mut large_ring, &mut scratch)
            .unwrap();

        let mut valence_encoder = ValencePacketEncoder::new();
        valence_encoder.set_compression(compression);
        valence_encoder.append_packet(packet).unwrap();

        let encoder_res = unsafe { encoder_res.as_slice() };

        let valence_encoder_res = valence_encoder.take().to_vec();

        // to slice
        let valence_encoder_res = valence_encoder_res.as_slice();

        let encoder_res = hex::encode(encoder_res);
        let valence_encoder_res = hex::encode(valence_encoder_res);

        // add 0x
        let encoder_res = format!("0x{encoder_res}");
        let valence_encoder_res = format!("0x{valence_encoder_res}");

        assert_eq!(encoder_res, valence_encoder_res, "{msg}");
    }

    fn compare_pkt2<P: Packet + Encode>(
        packet1: &P,
        packet2: &P,
        compression: CompressionThreshold,
        msg: &str,
    ) {
        let mut large_ring = Ring::new(MAX_PACKET_SIZE * 2);

        let mut encoder = PacketEncoder::new(compression, Compression::new(4));

        let bump = Bump::new();
        let mut scratch = Scratch::from(&bump);

        let encoder_res1 = encoder
            .append_packet(packet1, &mut large_ring, &mut scratch)
            .unwrap();

        let mut valence_encoder = ValencePacketEncoder::new();
        valence_encoder.set_compression(compression);
        valence_encoder.append_packet(packet1).unwrap();

        let encoder_res2 = encoder
            .append_packet(packet2, &mut large_ring, &mut scratch)
            .unwrap();

        println!("encoder_res1: {encoder_res1:?}");
        let encoder_res1 = unsafe { encoder_res1.as_slice() };
        println!("encoder_res1: {encoder_res1:X?}");

        valence_encoder.append_packet(packet2).unwrap();

        println!("encoder_res2: {encoder_res2:?}");
        let encoder_res2 = unsafe { encoder_res2.as_slice() };
        println!("encoder_res2: {encoder_res2:X?}");

        let combined_res = encoder_res1
            .iter()
            .chain(encoder_res2)
            .copied()
            .collect::<Vec<u8>>();

        let valence_encoder_res = valence_encoder.take().to_vec();

        // to slice
        let valence_encoder_res = valence_encoder_res.as_slice();

        let encoder_res = hex::encode(combined_res);
        let valence_encoder_res = hex::encode(valence_encoder_res);

        // add 0x
        let encoder_res = format!("0x{encoder_res}");
        let valence_encoder_res = format!("0x{valence_encoder_res}");

        assert_eq!(encoder_res, valence_encoder_res, "{msg}");
    }

    #[test]
    fn test_uncompressed() {
        fn compare<P: Packet + Encode>(packet: &P, msg: &str) {
            compare_pkt(packet, CompressionThreshold::default(), msg);
        }

        let login = login::LoginHelloC2s {
            username: Bounded::default(),
            profile_id: None,
        };
        compare(&login, "Empty LoginHelloC2s");

        let login = login::LoginHelloC2s {
            username: Bounded("Emerald_Explorer"),
            profile_id: None,
        };
        compare(&login, "LoginHelloC2s with 'Emerald_Explorer'");
    }

    #[test]
    fn test_compressed2() {
        fn compare<P: Packet + Encode>(packet1: &P, packet2: &P, msg: &str) {
            compare_pkt2(packet1, packet2, CompressionThreshold(2), msg);
        }

        fn random_name(input: &mut String) {
            let length = fastrand::usize(..14);
            for _ in 0..length {
                let c = fastrand::alphanumeric();
                input.push(c);
            }
        }

        fastrand::seed(7);

        let mut name1 = String::new();
        let mut name2 = String::new();
        for idx in 0..1000 {
            random_name(&mut name1);
            random_name(&mut name2);

            let pkt1 = login::LoginHelloC2s {
                username: Bounded(&name1),
                profile_id: None,
            };

            let pkt2 = login::LoginHelloC2s {
                username: Bounded(&name2),
                profile_id: None,
            };

            compare(
                &pkt1,
                &pkt2,
                &format!("LoginHelloC2s with '{name1}' and '{name2}' on idx {idx}"),
            );

            name1.clear();
            name2.clear();
        }
    }

    #[test]
    fn test_compressed() {
        fn compare<P: Packet + Encode>(packet: &P, msg: &str) {
            compare_pkt(packet, CompressionThreshold(10), msg);
        }

        fn random_name(input: &mut String) {
            let length = fastrand::usize(..14);
            for _ in 0..length {
                let c = fastrand::alphanumeric();
                input.push(c);
            }
        }

        let login = login::LoginHelloC2s {
            username: Bounded::default(),
            profile_id: None,
        };
        compare(&login, "Empty LoginHelloC2s");

        let login = login::LoginHelloC2s {
            username: Bounded("Emerald_Explorer"),
            profile_id: None,
        };
        compare(&login, "LoginHelloC2s with 'Emerald_Explorer'");

        fastrand::seed(7);

        let mut name = String::new();
        for _ in 0..1000 {
            random_name(&mut name);

            let pkt = login::LoginHelloC2s {
                username: Bounded(&name),
                profile_id: None,
            };

            compare(&pkt, &format!("LoginHelloC2s with '{name}'"));

            name.clear();
        }
    }

    #[test]
    fn test_compressed_very_small_double() {
        fn compare<P: Packet + Encode>(packet: &P, msg: &str) {
            compare_pkt(packet, CompressionThreshold(2), msg);
        }

        fn random_name(input: &mut String) {
            let length = fastrand::usize(..14);
            for _ in 0..length {
                let c = fastrand::alphanumeric();
                input.push(c);
            }
        }

        let login = login::LoginHelloC2s {
            username: Bounded::default(),
            profile_id: None,
        };
        compare(&login, "Empty LoginHelloC2s");

        let login = login::LoginHelloC2s {
            username: Bounded("Emerald_Explorer"),
            profile_id: None,
        };
        compare(&login, "LoginHelloC2s with 'Emerald_Explorer'");

        fastrand::seed(7);

        let mut name = String::new();
        for _ in 0..1000 {
            random_name(&mut name);

            let pkt = login::LoginHelloC2s {
                username: Bounded(&name),
                profile_id: None,
            };

            compare(&pkt, &format!("LoginHelloC2s with '{name}'"));

            name.clear();
        }
    }
}
