use std::io::Write;
use std::path::Path;

const OPUS_SAMPLE_RATE: u64 = 48_000;

const ID_SEGMENT: u64 = 0x18538067;
const ID_TIMECODE_SCALE: u64 = 0x2AD7B1;
const ID_TRACKS: u64 = 0x1654AE6B;
const ID_TRACK_ENTRY: u64 = 0xAE;
const ID_TRACK_NUMBER: u64 = 0xD7;
const ID_TRACK_TYPE: u64 = 0x83;
const ID_CODEC_PRIVATE: u64 = 0x63A2;
const ID_CLUSTER: u64 = 0x1F43B675;
const ID_CLUSTER_TIMESTAMP: u64 = 0xE7;
const ID_SIMPLE_BLOCK: u64 = 0xA3;
const ID_BLOCK_GROUP: u64 = 0xA0;
const ID_BLOCK: u64 = 0xA1;

const TRACK_TYPE_AUDIO: u64 = 2;

#[derive(Debug, Clone)]
pub struct OpusHead {
    pub channels: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub data: Vec<u8>,
    pub timestamp: i64,
}

pub struct DemuxedOpus {
    pub head: OpusHead,
    pub packets: Vec<Packet>,
    pub timecode_scale: u64,
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn vint_value_mask(width: usize) -> u8 {
        ((1u16 << (8 - width)) - 1) as u8
    }

    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn byte(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn vint_width(first: u8) -> Option<usize> {
        match first {
            0x80..=0xFE => Some(1),
            0x40..=0x7F => Some(2),
            0x20..=0x3F => Some(3),
            0x10..=0x1F => Some(4),
            0x08..=0x0F => Some(5),
            0x04..=0x07 => Some(6),
            0x02..=0x03 => Some(7),
            0x00..=0x01 => Some(8),
            _ => None,
        }
    }

    fn read_id(&mut self) -> Option<u64> {
        let first = self.byte()?;
        let width = Self::vint_width(first)?;
        if self.remaining() < width {
            return None;
        }
        let mut value = 0u64;
        for i in 0..width {
            value = (value << 8) | self.buf[self.pos + i] as u64;
        }
        self.pos += width;
        Some(value)
    }

    fn read_size(&mut self) -> Option<Option<u64>> {
        let first = self.byte()?;
        let width = Self::vint_width(first)?;
        if self.remaining() < width {
            return None;
        }
        let mut value = u64::from(first & Self::vint_value_mask(width));
        for i in 1..width {
            value = (value << 8) | self.buf[self.pos + i] as u64;
        }
        self.pos += width;
        let marker = 1u64.checked_shl(7u32 * width as u32).unwrap_or(0);
        if marker != 0 && value == marker - 1 {
            None
        } else {
            Some(Some(value))
        }
    }

    fn slice(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.remaining() < len {
            return None;
        }
        let out = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Some(out)
    }

    fn uint(&mut self, len: usize) -> Option<u64> {
        Some(
            self.slice(len)?
                .iter()
                .fold(0u64, |a, &b| (a << 8) | b as u64),
        )
    }

    fn track_vint(&mut self) -> Option<u64> {
        let first = self.byte()?;
        let width = Self::vint_width(first)?;
        if self.remaining() < width {
            return None;
        }
        let mut value = u64::from(first & Self::vint_value_mask(width));
        for i in 1..width {
            value = (value << 8) | self.buf[self.pos + i] as u64;
        }
        self.pos += width;
        Some(value)
    }
}

fn parse_opus_head(private: &[u8]) -> Option<OpusHead> {
    if private.len() < 19 || &private[..8] != b"OpusHead" {
        return None;
    }
    Some(OpusHead {
        channels: private[9],
        pre_skip: u16::from_le_bytes([private[10], private[11]]),
        input_sample_rate: u32::from_le_bytes([private[12], private[13], private[14], private[15]]),
    })
}

struct Walk<'a> {
    head: Option<OpusHead>,
    audio_track: u64,
    packets: Vec<Packet>,
    timecode_scale: u64,
    _buf: std::marker::PhantomData<&'a ()>,
}

impl<'a> Walk<'a> {
    fn new() -> Self {
        Self {
            head: None,
            audio_track: 0,
            packets: Vec::new(),
            timecode_scale: 1_000_000,
            _buf: std::marker::PhantomData,
        }
    }

    fn track_entry(&mut self, body: &[u8]) {
        let mut c = Cursor::new(body);
        while c.remaining() > 0 {
            let Some(id) = c.read_id() else { break };
            let Some(Some(size)) = c.read_size() else {
                break;
            };
            let size = size as usize;
            let start = c.pos;
            if start + size > body.len() {
                break;
            }
            match id {
                ID_TRACK_NUMBER => {
                    if let Some(v) = c.uint(size) {
                        self.audio_track = v;
                    }
                }
                ID_TRACK_TYPE => {
                    if c.uint(size) != Some(TRACK_TYPE_AUDIO) {
                        self.audio_track = u64::MAX;
                    }
                }
                ID_CODEC_PRIVATE => {
                    if let Some(private) = c.slice(size)
                        && self.head.is_none()
                    {
                        self.head = parse_opus_head(private);
                    }
                }
                ID_TRACK_ENTRY => {
                    let nested = &body[start..start + size];
                    self.track_entry(nested);
                }
                _ => {}
            }
            c.pos = start + size;
        }
    }

    fn block(&mut self, body: &[u8], cluster_ts: i64) {
        let mut c = Cursor::new(body);
        let Some(track) = c.track_vint() else { return };
        if track != self.audio_track {
            return;
        }
        let Some(tc) = c.slice(2) else { return };
        if c.slice(1).is_none() {
            return;
        }
        let timecode = i16::from_be_bytes([tc[0], tc[1]]) as i64;
        let Some(data) = c.slice(c.remaining()) else {
            return;
        };
        if data.is_empty() {
            return;
        }
        self.packets.push(Packet {
            data: data.to_vec(),
            timestamp: cluster_ts + timecode,
        });
    }

    fn block_group(&mut self, body: &[u8], cluster_ts: i64) {
        let mut c = Cursor::new(body);
        while c.remaining() > 0 {
            let Some(id) = c.read_id() else { break };
            let Some(Some(size)) = c.read_size() else {
                break;
            };
            let start = c.pos;
            if id == ID_BLOCK {
                let nested = &body[start..start + size as usize];
                self.block(nested, cluster_ts);
            }
            c.pos = start + size as usize;
        }
    }

    fn cluster(&mut self, body: &[u8]) {
        let mut c = Cursor::new(body);
        let mut cluster_ts: i64 = 0;
        while c.remaining() > 0 {
            let Some(id) = c.read_id() else { break };
            let Some(Some(size)) = c.read_size() else {
                break;
            };
            let size = size as usize;
            let start = c.pos;
            if start + size > body.len() {
                break;
            }
            match id {
                ID_CLUSTER_TIMESTAMP => {
                    if let Some(v) = c.uint(size) {
                        cluster_ts = v as i64;
                    }
                }
                ID_SIMPLE_BLOCK => {
                    let nested = &body[start..start + size];
                    self.block(nested, cluster_ts);
                }
                ID_BLOCK_GROUP => {
                    let nested = &body[start..start + size];
                    self.block_group(nested, cluster_ts);
                }
                _ => {}
            }
            c.pos = start + size;
        }
    }

    fn segment(&mut self, body: &[u8]) {
        let mut c = Cursor::new(body);
        while c.remaining() > 0 {
            let Some(id) = c.read_id() else { break };
            let size = match c.read_size() {
                Some(Some(s)) => s as usize,
                Some(None) => c.remaining(),
                None => break,
            };
            let start = c.pos;
            if start + size > body.len() {
                break;
            }
            match id {
                ID_TIMECODE_SCALE => {
                    if let Some(v) = c.uint(size) {
                        self.timecode_scale = v.max(1);
                    }
                }
                ID_TRACKS => {
                    let nested = &body[start..start + size];
                    self.track_entry(nested);
                }
                ID_CLUSTER => {
                    let nested = &body[start..start + size];
                    self.cluster(nested);
                }
                _ => {}
            }
            c.pos = start + size;
        }
    }
}

pub fn demux_bytes(data: &[u8]) -> Result<DemuxedOpus, String> {
    let mut walk = Walk::new();
    let mut probe = Cursor::new(data);

    while probe.remaining() > 0 {
        let Some(id) = probe.read_id() else { break };
        let Some(Some(size)) = probe.read_size() else {
            break;
        };
        let size = size as usize;
        if id == ID_SEGMENT {
            let rest = &data[probe.pos..];
            walk.segment(rest);
            break;
        }
        if size > probe.remaining() {
            break;
        }
        probe.pos += size;
    }

    let head = walk
        .head
        .ok_or_else(|| "Source has no Opus codec private data.".to_string())?;
    let mut packets = std::mem::take(&mut walk.packets);
    if packets.is_empty() {
        return Err("Source contains no Opus packets.".into());
    }
    packets.sort_by_key(|p| p.timestamp);
    Ok(DemuxedOpus {
        head,
        packets,
        timecode_scale: walk.timecode_scale,
    })
}

pub fn demux(path: &Path) -> Result<DemuxedOpus, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read source: {e}"))?;
    demux_bytes(&data)
}

fn sample_positions(packets: &[Packet], timecode_scale: u64) -> Vec<u64> {
    let Some(first) = packets.first() else {
        return Vec::new();
    };
    let base = first.timestamp;
    packets
        .iter()
        .map(|p| {
            let ticks = (p.timestamp - base) as f64;
            let ns = ticks * timecode_scale as f64;
            (ns * OPUS_SAMPLE_RATE as f64 / 1_000_000_000.0).round() as u64
        })
        .collect()
}

fn lacing(len: usize) -> Vec<u8> {
    if len == 0 {
        return vec![0];
    }
    let mut out = Vec::with_capacity(len / 255 + 1);
    let mut left = len;
    while left >= 255 {
        out.push(255u8);
        left -= 255;
    }
    out.push(left as u8);
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0;
    for &byte in data {
        crc ^= (byte as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
        }
    }
    crc
}

struct OggWriter {
    out: Vec<u8>,
    serial: u32,
    sequence: u32,
}

impl OggWriter {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            serial: 0x5649_4E4C,
            sequence: 0,
        }
    }

    fn page(&mut self, header_type: u8, granule: u64, packet: &[u8]) {
        let segments = lacing(packet.len());
        let start = self.out.len();
        self.out.extend_from_slice(b"OggS");
        self.out.push(0);
        self.out.push(header_type);
        self.out.extend_from_slice(&granule.to_le_bytes());
        self.out.extend_from_slice(&self.serial.to_le_bytes());
        self.out.extend_from_slice(&self.sequence.to_le_bytes());
        self.out.extend_from_slice(&[0, 0, 0, 0]);
        self.out.push(segments.len() as u8);
        self.out.extend_from_slice(&segments);
        self.out.extend_from_slice(packet);

        let crc = crc32(&self.out[start..]);
        let at = start + 22;
        self.out[at..at + 4].copy_from_slice(&crc.to_le_bytes());
        self.sequence += 1;
    }
}

fn opus_head_packet(head: &OpusHead) -> Vec<u8> {
    let mut v = Vec::with_capacity(19);
    v.extend_from_slice(b"OpusHead");
    v.push(1);
    v.push(head.channels);
    v.extend_from_slice(&head.pre_skip.to_le_bytes());
    v.extend_from_slice(&head.input_sample_rate.to_le_bytes());
    v.extend_from_slice(&0i16.to_le_bytes());
    v.push(if head.channels > 2 { 1 } else { 0 });
    v
}

fn opus_tags_packet(vendor: &str, comments: &[(String, String)]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"OpusTags");
    v.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    v.extend_from_slice(vendor.as_bytes());
    v.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for (key, value) in comments {
        let body = format!("{key}={value}");
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(body.as_bytes());
    }
    v
}

pub fn mux(
    head: &OpusHead,
    packets: &[Packet],
    comments: &[(String, String)],
    dest: &Path,
    timecode_scale: u64,
) -> Result<u64, String> {
    if packets.is_empty() {
        return Err("No packets to write.".into());
    }

    let mut writer = OggWriter::new();

    writer.page(0x02, 0, &opus_head_packet(head));
    writer.page(0x00, 0, &opus_tags_packet("Vynl", comments));

    let positions = sample_positions(packets, timecode_scale);
    let total = packets.len();

    let typical = if total > 1 {
        positions[total - 1] / (total as u64 - 1)
    } else {
        960
    };

    for (i, packet) in packets.iter().enumerate() {
        let last = i + 1 == total;
        let granule = if last {
            positions[total - 1]
                .saturating_add(typical)
                .saturating_sub(head.pre_skip as u64)
        } else {
            positions[i + 1]
        };
        writer.page(if last { 0x04 } else { 0x00 }, granule, &packet.data);
    }

    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut file =
        std::fs::File::create(dest).map_err(|e| format!("Failed to create {dest:?}: {e}"))?;
    file.write_all(&writer.out)
        .map_err(|e| format!("Failed to write {dest:?}: {e}"))?;
    Ok(writer.out.len() as u64)
}

pub fn remux(src: &Path, dest: &Path) -> Result<u64, String> {
    let demuxed = demux(src)?;
    mux(
        &demuxed.head,
        &demuxed.packets,
        &[],
        dest,
        demuxed.timecode_scale,
    )
}
