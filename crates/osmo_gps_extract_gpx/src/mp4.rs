use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use time::Duration;

use crate::error::{Error, Result};

const MAX_MOOV_SIZE: u64 = 512 * 1024 * 1024;
const MAX_SAMPLE_SIZE: u32 = 64 * 1024 * 1024;
const MAX_SAMPLES: usize = 50_000_000;

#[derive(Debug, Clone, Copy)]
struct BoxHeader {
    kind: [u8; 4],
    offset: u64,
    header_len: u64,
    size: u64,
}

#[derive(Debug)]
struct BoxRef<'a> {
    kind: [u8; 4],
    data: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct StscEntry {
    first_chunk: u32,
    samples_per_chunk: u32,
}

#[derive(Debug, Clone, Copy)]
struct SttsEntry {
    sample_count: u32,
    sample_delta: u32,
}

#[derive(Debug, Default)]
struct Track {
    sample_entry: Option<[u8; 4]>,
    timescale: u32,
    sample_sizes: Vec<u32>,
    chunk_offsets: Vec<u64>,
    stsc: Vec<StscEntry>,
    stts: Vec<SttsEntry>,
}

#[derive(Debug, Clone, Copy)]
struct SampleRef {
    offset: u64,
    size: u32,
    time: Duration,
}

/// One `djmd` sample with its position on the track timeline.
pub(crate) struct MetadataSample {
    /// Decode time from the start of the track, from `stts`.
    pub(crate) time: Duration,
    pub(crate) data: Vec<u8>,
}

/// Read every sample of the first `djmd` track from an MP4 file.
pub(crate) fn read_djmd_samples(path: &Path) -> Result<Vec<MetadataSample>> {
    let mut file = File::open(path)?;
    let file_len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;

    let moov = read_moov(&mut file, file_len)?;
    let track = find_djmd_track(&moov)?;
    let refs = sample_refs(&track)?;
    read_samples(path, file_len, &refs)
}

fn read_moov(file: &mut File, file_len: u64) -> Result<Vec<u8>> {
    loop {
        let Some(header) = read_box_header(file, file_len)? else {
            return Err(Error::InvalidMp4("missing `moov` box".to_owned()));
        };
        if &header.kind == b"moov" {
            if header.size > MAX_MOOV_SIZE {
                return Err(Error::InvalidMp4(format!(
                    "`moov` box is too large ({} bytes)",
                    header.size
                )));
            }
            let payload_len = usize::try_from(header.size - header.header_len)
                .map_err(|_| Error::InvalidMp4("`moov` box does not fit in memory".to_owned()))?;
            let mut data = vec![0u8; payload_len];
            file.read_exact(&mut data)?;
            return Ok(data);
        }
        let next = header
            .offset
            .checked_add(header.size)
            .ok_or_else(|| Error::InvalidMp4("box offset overflow".to_owned()))?;
        file.seek(SeekFrom::Start(next))?;
    }
}

fn read_box_header<R: Read + Seek>(reader: &mut R, file_len: u64) -> Result<Option<BoxHeader>> {
    let offset = reader.stream_position()?;
    if offset >= file_len {
        return Ok(None);
    }

    let mut header = [0u8; 8];
    reader.read_exact(&mut header)?;
    let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let kind = [header[4], header[5], header[6], header[7]];

    let (size, header_len) = match size32 {
        0 => (file_len - offset, 8),
        1 => {
            let mut large = [0u8; 8];
            reader.read_exact(&mut large)?;
            (u64::from_be_bytes(large), 16)
        }
        value => (u64::from(value), 8),
    };

    if size < header_len {
        return Err(Error::InvalidMp4(format!(
            "box {} at {offset} has invalid size {size}",
            fourcc(kind)
        )));
    }
    let end = offset
        .checked_add(size)
        .ok_or_else(|| Error::InvalidMp4("box offset overflow".to_owned()))?;
    if end > file_len {
        return Err(Error::InvalidMp4(format!(
            "box {} at {offset} extends past EOF",
            fourcc(kind)
        )));
    }

    Ok(Some(BoxHeader {
        kind,
        offset,
        header_len,
        size,
    }))
}

fn child_boxes(data: &[u8]) -> Result<Vec<BoxRef<'_>>> {
    let mut boxes = Vec::new();
    let mut pos = 0usize;

    while pos < data.len() {
        if data.len() - pos < 8 {
            return Err(Error::InvalidMp4("truncated box header".to_owned()));
        }
        let size32 = be_u32(&data[pos..pos + 4]);
        let kind = [data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]];

        let (size, header_len) = match size32 {
            0 => ((data.len() - pos) as u64, 8usize),
            1 => {
                if data.len() - pos < 16 {
                    return Err(Error::InvalidMp4(format!(
                        "truncated extended box header for {}",
                        fourcc(kind)
                    )));
                }
                (be_u64(&data[pos + 8..pos + 16]), 16)
            }
            value => (u64::from(value), 8),
        };

        if size < header_len as u64 {
            return Err(Error::InvalidMp4(format!(
                "box {} at {pos} has invalid size {size}",
                fourcc(kind)
            )));
        }
        let size = usize::try_from(size)
            .map_err(|_| Error::InvalidMp4("box does not fit in memory".to_owned()))?;
        let end = pos
            .checked_add(size)
            .ok_or_else(|| Error::InvalidMp4("box offset overflow".to_owned()))?;
        if end > data.len() {
            return Err(Error::InvalidMp4(format!(
                "box {} at {pos} extends past its parent",
                fourcc(kind)
            )));
        }

        boxes.push(BoxRef {
            kind,
            data: &data[pos + header_len..end],
        });
        pos = end;
    }

    Ok(boxes)
}

fn find_djmd_track(moov: &[u8]) -> Result<Track> {
    let mut entries = Vec::new();

    for child in child_boxes(moov)? {
        if &child.kind != b"trak" {
            continue;
        }
        let track = parse_trak(child.data)?;
        if let Some(entry) = track.sample_entry {
            if &entry == b"djmd" {
                return Ok(track);
            }
            entries.push(fourcc(entry));
        }
    }

    Err(Error::MissingMetadataTrack(entries.join(", ")))
}

fn parse_trak(data: &[u8]) -> Result<Track> {
    let mut track = Track::default();

    for child in child_boxes(data)? {
        if &child.kind != b"mdia" {
            continue;
        }
        for media in child_boxes(child.data)? {
            match &media.kind {
                b"mdhd" => track.timescale = parse_mdhd(media.data)?,
                b"minf" => parse_minf(media.data, &mut track)?,
                _ => {}
            }
        }
    }

    Ok(track)
}

fn parse_mdhd(data: &[u8]) -> Result<u32> {
    // Full box: version/flags, creation and modification times, timescale.
    // Version 1 widens the two timestamps to 64 bits.
    let (offset, minimum_len) = match data.first() {
        Some(0) => (12usize, 20usize),
        Some(1) => (20usize, 32usize),
        Some(version) => {
            return Err(Error::InvalidMp4(format!(
                "unsupported `mdhd` version {version}"
            )));
        }
        None => return Err(Error::InvalidMp4("`mdhd` box is empty".to_owned())),
    };
    if data.len() < minimum_len {
        return Err(Error::InvalidMp4("`mdhd` box is too short".to_owned()));
    }

    let timescale = be_u32(&data[offset..offset + 4]);
    if timescale == 0 {
        return Err(Error::InvalidMp4("`mdhd` timescale is zero".to_owned()));
    }
    Ok(timescale)
}

fn parse_minf(data: &[u8], track: &mut Track) -> Result<()> {
    for child in child_boxes(data)? {
        if &child.kind == b"stbl" {
            parse_stbl(child.data, track)?;
        }
    }
    Ok(())
}

fn parse_stbl(data: &[u8], track: &mut Track) -> Result<()> {
    for child in child_boxes(data)? {
        match &child.kind {
            b"stsd" => track.sample_entry = Some(parse_stsd(child.data)?),
            b"stsz" => track.sample_sizes = parse_stsz(child.data)?,
            b"stco" => track.chunk_offsets = parse_stco(child.data)?,
            b"co64" => track.chunk_offsets = parse_co64(child.data)?,
            b"stsc" => track.stsc = parse_stsc(child.data)?,
            b"stts" => track.stts = parse_stts(child.data)?,
            _ => {}
        }
    }
    Ok(())
}

fn parse_stsd(data: &[u8]) -> Result<[u8; 4]> {
    if data.len() < 16 {
        return Err(Error::InvalidMp4("`stsd` box is too short".to_owned()));
    }
    let entry_count = be_u32(&data[4..8]);
    if entry_count == 0 {
        return Err(Error::InvalidMp4("`stsd` has no sample entry".to_owned()));
    }
    Ok([data[12], data[13], data[14], data[15]])
}

fn parse_stsz(data: &[u8]) -> Result<Vec<u32>> {
    if data.len() < 12 {
        return Err(Error::InvalidMp4("`stsz` box is too short".to_owned()));
    }
    let sample_size = be_u32(&data[4..8]);
    let count = be_u32(&data[8..12]) as usize;
    if count > MAX_SAMPLES {
        return Err(Error::InvalidMp4(format!(
            "`stsz` sample count {count} is too large"
        )));
    }

    if sample_size != 0 {
        return Ok(vec![sample_size; count]);
    }

    let needed = 12usize
        .checked_add(
            count
                .checked_mul(4)
                .ok_or_else(|| Error::InvalidMp4("`stsz` size table overflow".to_owned()))?,
        )
        .ok_or_else(|| Error::InvalidMp4("`stsz` size table overflow".to_owned()))?;
    if data.len() < needed {
        return Err(Error::InvalidMp4("truncated `stsz` size table".to_owned()));
    }

    let mut sizes = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 12 + index * 4;
        sizes.push(be_u32(&data[offset..offset + 4]));
    }
    Ok(sizes)
}

fn parse_stco(data: &[u8]) -> Result<Vec<u64>> {
    if data.len() < 8 {
        return Err(Error::InvalidMp4("`stco` box is too short".to_owned()));
    }
    let count = be_u32(&data[4..8]) as usize;
    if count > MAX_SAMPLES {
        return Err(Error::InvalidMp4(format!(
            "`stco` entry count {count} is too large"
        )));
    }
    let needed = 8usize
        .checked_add(
            count
                .checked_mul(4)
                .ok_or_else(|| Error::InvalidMp4("`stco` table overflow".to_owned()))?,
        )
        .ok_or_else(|| Error::InvalidMp4("`stco` table overflow".to_owned()))?;
    if data.len() < needed {
        return Err(Error::InvalidMp4(
            "truncated `stco` offset table".to_owned(),
        ));
    }

    let mut offsets = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 8 + index * 4;
        offsets.push(u64::from(be_u32(&data[offset..offset + 4])));
    }
    Ok(offsets)
}

fn parse_co64(data: &[u8]) -> Result<Vec<u64>> {
    if data.len() < 8 {
        return Err(Error::InvalidMp4("`co64` box is too short".to_owned()));
    }
    let count = be_u32(&data[4..8]) as usize;
    if count > MAX_SAMPLES {
        return Err(Error::InvalidMp4(format!(
            "`co64` entry count {count} is too large"
        )));
    }
    let needed = 8usize
        .checked_add(
            count
                .checked_mul(8)
                .ok_or_else(|| Error::InvalidMp4("`co64` table overflow".to_owned()))?,
        )
        .ok_or_else(|| Error::InvalidMp4("`co64` table overflow".to_owned()))?;
    if data.len() < needed {
        return Err(Error::InvalidMp4(
            "truncated `co64` offset table".to_owned(),
        ));
    }

    let mut offsets = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 8 + index * 8;
        offsets.push(be_u64(&data[offset..offset + 8]));
    }
    Ok(offsets)
}

fn parse_stsc(data: &[u8]) -> Result<Vec<StscEntry>> {
    if data.len() < 8 {
        return Err(Error::InvalidMp4("`stsc` box is too short".to_owned()));
    }
    let count = be_u32(&data[4..8]) as usize;
    if count == 0 {
        return Err(Error::InvalidMp4("`stsc` has no entries".to_owned()));
    }
    if count > MAX_SAMPLES {
        return Err(Error::InvalidMp4(format!(
            "`stsc` entry count {count} is too large"
        )));
    }
    let needed = 8usize
        .checked_add(
            count
                .checked_mul(12)
                .ok_or_else(|| Error::InvalidMp4("`stsc` table overflow".to_owned()))?,
        )
        .ok_or_else(|| Error::InvalidMp4("`stsc` table overflow".to_owned()))?;
    if data.len() < needed {
        return Err(Error::InvalidMp4(
            "truncated `stsc` sample-to-chunk table".to_owned(),
        ));
    }

    let mut entries: Vec<StscEntry> = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 8 + index * 12;
        let first_chunk = be_u32(&data[offset..offset + 4]);
        let samples_per_chunk = be_u32(&data[offset + 4..offset + 8]);

        if first_chunk == 0 || samples_per_chunk == 0 {
            return Err(Error::InvalidMp4(
                "`stsc` has a zero chunk/sample count".to_owned(),
            ));
        }
        if let Some(previous) = entries.last() {
            if first_chunk <= previous.first_chunk {
                return Err(Error::InvalidMp4(
                    "`stsc` entries are not sorted".to_owned(),
                ));
            }
        } else if first_chunk != 1 {
            return Err(Error::InvalidMp4(
                "`stsc` does not start at chunk 1".to_owned(),
            ));
        }

        entries.push(StscEntry {
            first_chunk,
            samples_per_chunk,
        });
    }

    Ok(entries)
}

fn parse_stts(data: &[u8]) -> Result<Vec<SttsEntry>> {
    if data.len() < 8 {
        return Err(Error::InvalidMp4("`stts` box is too short".to_owned()));
    }
    let count = be_u32(&data[4..8]) as usize;
    if count > MAX_SAMPLES {
        return Err(Error::InvalidMp4(format!(
            "`stts` entry count {count} is too large"
        )));
    }
    let needed = 8usize
        .checked_add(
            count
                .checked_mul(8)
                .ok_or_else(|| Error::InvalidMp4("`stts` table overflow".to_owned()))?,
        )
        .ok_or_else(|| Error::InvalidMp4("`stts` table overflow".to_owned()))?;
    if data.len() < needed {
        return Err(Error::InvalidMp4(
            "truncated `stts` time-to-sample table".to_owned(),
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 8 + index * 8;
        let sample_count = be_u32(&data[offset..offset + 4]);
        let sample_delta = be_u32(&data[offset + 4..offset + 8]);
        if sample_count == 0 {
            return Err(Error::InvalidMp4(
                "`stts` has a zero sample count".to_owned(),
            ));
        }
        entries.push(SttsEntry {
            sample_count,
            sample_delta,
        });
    }

    Ok(entries)
}

/// Decode time of every sample, taken from the track `stts` table.
fn sample_times(track: &Track, sample_count: usize) -> Result<Vec<Duration>> {
    if track.timescale == 0 {
        return Err(Error::InvalidMp4(
            "metadata track has no `mdhd` timescale".to_owned(),
        ));
    }
    if track.stts.is_empty() {
        return Err(Error::InvalidMp4(
            "metadata track has no `stts` table".to_owned(),
        ));
    }

    let mut times = Vec::with_capacity(sample_count);
    let mut ticks = 0u64;
    for entry in &track.stts {
        for _ in 0..entry.sample_count {
            if times.len() == sample_count {
                return Err(Error::InvalidMp4(
                    "`stts` covers more samples than `stsz`".to_owned(),
                ));
            }
            times.push(ticks_to_duration(ticks, track.timescale)?);
            ticks = ticks
                .checked_add(u64::from(entry.sample_delta))
                .ok_or_else(|| Error::InvalidMp4("`stts` decode time overflow".to_owned()))?;
        }
    }

    if times.len() != sample_count {
        return Err(Error::InvalidMp4(format!(
            "`stsz` has {sample_count} samples but `stts` covers {}",
            times.len()
        )));
    }

    Ok(times)
}

fn ticks_to_duration(ticks: u64, timescale: u32) -> Result<Duration> {
    let nanos = u128::from(ticks) * 1_000_000_000 / u128::from(timescale);
    let nanos = i64::try_from(nanos)
        .map_err(|_| Error::InvalidMp4("`stts` decode time is too large".to_owned()))?;
    Ok(Duration::nanoseconds(nanos))
}

fn sample_refs(track: &Track) -> Result<Vec<SampleRef>> {
    if track.sample_sizes.is_empty() {
        return Err(Error::InvalidMp4(
            "metadata track has no samples".to_owned(),
        ));
    }
    if track.chunk_offsets.is_empty() {
        return Err(Error::InvalidMp4(
            "metadata track has no chunk offsets".to_owned(),
        ));
    }
    if track.stsc.is_empty() {
        return Err(Error::InvalidMp4(
            "metadata track has no `stsc` table".to_owned(),
        ));
    }

    let mut refs = Vec::with_capacity(track.sample_sizes.len());
    let mut sample_index = 0usize;
    let mut stsc_index = 0usize;
    let times = sample_times(track, track.sample_sizes.len())?;

    for (chunk_index, &chunk_offset) in track.chunk_offsets.iter().enumerate() {
        let chunk_number = u32::try_from(chunk_index + 1)
            .map_err(|_| Error::InvalidMp4("too many chunks".to_owned()))?;
        while stsc_index + 1 < track.stsc.len()
            && track.stsc[stsc_index + 1].first_chunk <= chunk_number
        {
            stsc_index += 1;
        }

        let entry = track
            .stsc
            .get(stsc_index)
            .ok_or_else(|| Error::InvalidMp4("`stsc` lookup failed".to_owned()))?;
        if entry.first_chunk > chunk_number {
            return Err(Error::InvalidMp4(
                "`stsc` does not cover the first chunk".to_owned(),
            ));
        }

        let mut offset = chunk_offset;
        for _ in 0..entry.samples_per_chunk {
            let Some(&size) = track.sample_sizes.get(sample_index) else {
                return Err(Error::InvalidMp4(
                    "`stsc` references more samples than `stsz`".to_owned(),
                ));
            };
            if size > MAX_SAMPLE_SIZE {
                return Err(Error::InvalidMp4(format!(
                    "sample {sample_index} is too large ({size} bytes)"
                )));
            }
            refs.push(SampleRef {
                offset,
                size,
                time: times[sample_index],
            });
            offset = offset
                .checked_add(u64::from(size))
                .ok_or_else(|| Error::InvalidMp4("sample offset overflow".to_owned()))?;
            sample_index += 1;
        }
    }

    if sample_index != track.sample_sizes.len() {
        return Err(Error::InvalidMp4(format!(
            "`stsz` has {} samples but chunk tables cover {sample_index}",
            track.sample_sizes.len()
        )));
    }

    Ok(refs)
}

fn read_samples(path: &Path, file_len: u64, refs: &[SampleRef]) -> Result<Vec<MetadataSample>> {
    let mut file = File::open(path)?;
    let mut samples = Vec::with_capacity(refs.len());

    for sample in refs {
        let end = sample
            .offset
            .checked_add(u64::from(sample.size))
            .ok_or_else(|| Error::InvalidMp4("sample offset overflow".to_owned()))?;
        if end > file_len {
            return Err(Error::InvalidMp4(format!(
                "sample at {} extends past EOF",
                sample.offset
            )));
        }

        file.seek(SeekFrom::Start(sample.offset))?;
        let mut data = vec![0u8; sample.size as usize];
        file.read_exact(&mut data)?;
        samples.push(MetadataSample {
            time: sample.time,
            data,
        });
    }

    Ok(samples)
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn be_u64(bytes: &[u8]) -> u64 {
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn fourcc(value: [u8; 4]) -> String {
    value
        .iter()
        .map(|&byte| {
            if byte.is_ascii_graphic() {
                char::from(byte)
            } else {
                '?'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + payload.len()).expect("test box fits in u32");
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    fn full_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + payload.len());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(payload);
        boxed(kind, &body)
    }

    fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
        parts.iter().flatten().copied().collect()
    }

    fn sample_table(sizes: &[u32], offsets: &[u32], stsc: &[(u32, u32)]) -> Vec<u8> {
        let mut stsd = vec![0, 0, 0, 1];
        stsd.extend_from_slice(&20u32.to_be_bytes());
        stsd.extend_from_slice(b"djmd");
        stsd.extend_from_slice(&[0u8; 8]);

        let mut stsz = Vec::new();
        stsz.extend_from_slice(&0u32.to_be_bytes());
        stsz.extend_from_slice(&(sizes.len() as u32).to_be_bytes());
        for size in sizes {
            stsz.extend_from_slice(&size.to_be_bytes());
        }

        let mut stco = Vec::new();
        stco.extend_from_slice(&(offsets.len() as u32).to_be_bytes());
        for offset in offsets {
            stco.extend_from_slice(&offset.to_be_bytes());
        }

        let mut stsc_body = Vec::new();
        stsc_body.extend_from_slice(&(stsc.len() as u32).to_be_bytes());
        for (first_chunk, samples_per_chunk) in stsc {
            stsc_body.extend_from_slice(&first_chunk.to_be_bytes());
            stsc_body.extend_from_slice(&samples_per_chunk.to_be_bytes());
            stsc_body.extend_from_slice(&1u32.to_be_bytes());
        }

        concat(&[
            full_box(b"stsd", &stsd),
            full_box(b"stsz", &stsz),
            full_box(b"stco", &stco),
            full_box(b"stsc", &stsc_body),
        ])
    }

    fn stts_box(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for (sample_count, sample_delta) in entries {
            body.extend_from_slice(&sample_count.to_be_bytes());
            body.extend_from_slice(&sample_delta.to_be_bytes());
        }
        full_box(b"stts", &body)
    }

    fn mdhd_box(timescale: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        full_box(b"mdhd", &body)
    }

    fn moov_with_track(stbl: &[u8], stts: &[(u32, u32)], timescale: u32) -> Vec<u8> {
        let stbl = boxed(b"stbl", &concat(&[stbl.to_vec(), stts_box(stts)]));
        let minf = boxed(b"minf", &stbl);
        let mdia = boxed(b"mdia", &concat(&[mdhd_box(timescale), minf]));
        let trak = boxed(b"trak", &mdia);
        boxed(b"moov", &trak)
    }

    #[test]
    fn maps_samples_across_chunks() {
        let stbl = sample_table(&[3, 4, 5], &[100, 200], &[(1, 2), (2, 1)]);
        let moov = moov_with_track(&stbl, &[(3, 1001)], 30_000);
        let track = find_djmd_track(&moov[8..]).expect("djmd track");
        let refs = sample_refs(&track).expect("sample refs");

        let actual: Vec<(u64, u32)> = refs.iter().map(|s| (s.offset, s.size)).collect();
        assert_eq!(actual, vec![(100, 3), (103, 4), (200, 5)]);
    }

    #[test]
    fn takes_sample_times_from_stts() {
        let stbl = sample_table(&[3, 4, 5, 6], &[100, 200], &[(1, 2), (2, 2)]);
        // Two samples one second apart, then two more half a second apart.
        let moov = moov_with_track(&stbl, &[(2, 1000), (2, 500)], 1000);
        let track = find_djmd_track(&moov[8..]).expect("djmd track");
        let refs = sample_refs(&track).expect("sample refs");

        let times: Vec<Duration> = refs.iter().map(|sample| sample.time).collect();
        assert_eq!(
            times,
            vec![
                Duration::ZERO,
                Duration::seconds(1),
                Duration::seconds(2),
                Duration::milliseconds(2500)
            ]
        );
    }

    #[test]
    fn rejects_stts_that_does_not_cover_every_sample() {
        let stbl = sample_table(&[3, 4, 5], &[100, 200], &[(1, 2), (2, 1)]);
        let moov = moov_with_track(&stbl, &[(2, 1000)], 1000);
        let track = find_djmd_track(&moov[8..]).expect("djmd track");

        let error = sample_refs(&track).expect_err("sample count mismatch");
        assert!(error.to_string().contains("`stts` covers 2"));
    }

    #[test]
    fn rejects_zero_timescale() {
        let stbl = sample_table(&[3], &[100], &[(1, 1)]);
        let moov = moov_with_track(&stbl, &[(1, 1000)], 0);

        let error = find_djmd_track(&moov[8..]).expect_err("zero timescale");
        assert!(error.to_string().contains("timescale"));
    }

    #[test]
    fn rejects_missing_djmd_track() {
        let stbl = sample_table(&[1], &[100], &[(1, 1)]);
        let moov = moov_with_track(&stbl, &[(1, 1000)], 1000);
        let mut broken = moov.clone();
        let position = broken
            .windows(4)
            .position(|window| window == b"djmd")
            .expect("djmd marker");
        broken[position..position + 4].copy_from_slice(b"dbgi");

        let error = find_djmd_track(&broken[8..]).expect_err("missing track");
        assert!(error.to_string().contains("dbgi"));
    }
}
