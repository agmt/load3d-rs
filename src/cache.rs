use crate::error::{Error, Res, Warnings, check, fail, utf8_valid_length};
use crate::matcher::match_pattern;
use crate::parse::push_sanitized_string_entry;
use crate::paths::open_file;
use crate::types::*;
use crate::xml::{XmlTag, load_xml, parse_uint32_radix};

const MAX_SKIP_SIZE: u64 = 0x4000_0000;
const GEOMETRY_CACHE_BUFFER_SIZE: usize = 512;
const CACHE_BUFFER_SIZE: usize = 128;

/// Indexed by `CacheDataFormat`
const CACHE_DATA_FORMAT_SIZE: [u32; 5] = [0, 4, 12, 8, 24];

const CACHE_INTERPRETATION_NAMES: [(CacheInterpretation, &str); 3] =
    [(CacheInterpretation::Points, "\\cpoints?"), (CacheInterpretation::VertexPosition, "\\cpositions?"), (CacheInterpretation::VertexNormal, "\\cnormals?")];

const fn cache_mc_tag(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

/// A skip may pass the end, from where reads give nothing, as with ufbx's stdio stream
struct Stream {
    data: Vec<u8>,
    pos: u64,
}

impl Stream {
    /// Like `ufbxi_stdio_read` in ufbx.c
    fn read(&mut self, dst: &mut [u8]) -> usize {
        let start = self.pos.min(self.data.len() as u64) as usize;
        let n = dst.len().min(self.data.len() - start);
        dst[..n].copy_from_slice(&self.data[start..start + n]);
        self.pos += n as u64;
        n
    }

    /// Like `ufbxi_stdio_skip` in ufbx.c
    fn skip(&mut self, size: u64) {
        self.pos = self.pos.saturating_add(size);
    }

    fn remaining(&self) -> u64 {
        (self.data.len() as u64).saturating_sub(self.pos)
    }
}

/// Like `ufbxi_alloc_size` in ufbx.c, its failures only: past `SIZE_MAX / 2` bytes without a description, else out of memory with `info`
fn reserve<T>(v: &mut Vec<T>, additional: usize, info: &str) -> Res<()> {
    check!(additional.checked_mul(size_of::<T>()).is_some_and(|total| total <= usize::MAX / 2));
    if v.try_reserve_exact(additional).is_err() {
        return Err(Error::new("Out of memory").set_err_info(info));
    }
    Ok(())
}

/// `ufbxi_cache_tmp_channel` in ufbx.c
#[derive(Debug, Clone, Default)]
struct TmpChannel {
    name: String,
    interpretation: String,
    sample_rate: u32,
    start_time: u32,
    end_time: u32,
    current_time: u32,
    consecutive_fails: u32,
    try_load: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheXmlType {
    None,
    FilePerFrame,
    SingleFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheXmlFormat {
    None,
    Mcc,
    Mcx,
}

struct CacheContext<'a> {
    opts: &'a GeometryCacheOpts,
    frames_per_second: f64,
    unicode_error_handling: UnicodeErrorHandling,
    warnings: &'a mut Warnings,

    channels: Vec<TmpChannel>,
    frames: Vec<CacheFrame>,

    stream_filename: String,
    stream: Stream,

    mc_for8: bool,

    xml_loaded: bool,
    xml_filename: String,
    xml_ticks_per_frame: u32,
    xml_type: CacheXmlType,
    xml_format: CacheXmlFormat,

    channel_name: String,
    name_cap: usize,

    file_offset: u64,
    pos: usize,
    pos_end: usize,

    cache: GeometryCache,

    /// Kept between reads and files: past a short read ufbx serves its stale bytes, and so does this
    buffer: [u8; CACHE_BUFFER_SIZE],
}

impl<'a> CacheContext<'a> {
    fn new(opts: &'a GeometryCacheOpts, frames_per_second: f64, unicode_error_handling: UnicodeErrorHandling, warnings: &'a mut Warnings) -> Self {
        CacheContext {
            opts,
            frames_per_second,
            unicode_error_handling,
            warnings,
            channels: Vec::new(),
            frames: Vec::new(),
            stream_filename: String::new(),
            stream: Stream { data: Vec::new(), pos: 0 },
            mc_for8: false,
            xml_loaded: false,
            xml_filename: String::new(),
            xml_ticks_per_frame: 0,
            xml_type: CacheXmlType::None,
            xml_format: CacheXmlFormat::None,
            channel_name: String::new(),
            name_cap: 0,
            file_offset: 0,
            pos: 0,
            pos_end: 0,
            cache: GeometryCache::default(),
            buffer: [0; CACHE_BUFFER_SIZE],
        }
    }

    /// Invalid UTF-8 repaired, as ufbx's string pool does
    fn push_string_place_str(&mut self, s: &[u8]) -> Res<String> {
        let valid = utf8_valid_length(s);
        if valid == s.len() {
            return Ok(String::from_utf8_lossy(s).into_owned());
        }
        push_sanitized_string_entry(s, valid, self.unicode_error_handling, self.warnings)
    }

    /// Zeros past the end where `allow_eof`
    fn cache_read(&mut self, dst: &mut [u8], allow_eof: bool) -> Res<()> {
        let mut size = dst.len();
        let buffered = (self.pos_end - self.pos).min(size);
        dst[..buffered].copy_from_slice(&self.buffer[self.pos..self.pos + buffered]);
        self.pos += buffered;
        size -= buffered;
        self.file_offset += buffered as u64;
        if size == 0 {
            return Ok(());
        }
        let mut d = buffered;

        if size >= CACHE_BUFFER_SIZE {
            let num_read = self.stream.read(&mut dst[d..d + size]);
            if !allow_eof {
                check!(num_read == size, "Truncated file");
            }
            self.file_offset += num_read as u64;
            size -= num_read;
            d += num_read;
        } else {
            let num_read = self.stream.read(&mut self.buffer);
            if !allow_eof {
                check!(num_read >= size, "Truncated file");
            }
            self.pos = 0;
            self.pos_end = CACHE_BUFFER_SIZE;

            dst[d..d + size].copy_from_slice(&self.buffer[..size]);
            self.pos += size;
            self.file_offset += size as u64;

            let num_written = size.min(num_read);
            size -= num_written;
            d += num_written;
        }

        dst[d..d + size].fill(0);
        Ok(())
    }

    /// With `ufbxi_grow_array_size` in ufbx.c: kept only for its failures, a first allocation naming the allocator in the error, a reallocation not
    fn grow_array(&mut self, n: usize) -> Res<()> {
        if n <= self.name_cap {
            return Ok(());
        }
        let new_n = (self.name_cap * 2).max(n);
        reserve(&mut Vec::<u8>::new(), new_n, if self.name_cap == 0 { "temp" } else { "" })?;
        self.name_cap = new_n;
        Ok(())
    }

    /// Refused before allocating where the stream holds fewer than `size` bytes
    fn read_vec(&mut self, size: usize) -> Res<Vec<u8>> {
        check!(size as u64 <= (self.pos_end - self.pos) as u64 + self.stream.remaining(), "Truncated file");
        let mut dst = vec![0; size];
        self.cache_read(&mut dst, false)?;
        Ok(dst)
    }

    /// Seeks succeed past the end, as with ufbx's stdio stream
    fn cache_skip(&mut self, size: u64) -> Res<()> {
        let mut size = size;
        self.file_offset = self.file_offset.wrapping_add(size);

        let buffered = ((self.pos_end - self.pos) as u64).min(size);
        self.pos += buffered as usize;
        size -= buffered;

        while size >= MAX_SKIP_SIZE {
            size -= MAX_SKIP_SIZE;
            self.stream.skip(MAX_SKIP_SIZE - 1);
            let mut single_byte = [0u8; 1];
            let num_read = self.stream.read(&mut single_byte);
            check!(num_read == 1, "Truncated file");
        }
        if size > 0 {
            self.stream.skip(size);
        }
        Ok(())
    }

    fn cache_mc_read_tag(&mut self) -> Res<u32> {
        let mut buf = [0u8; 4];
        self.cache_read(&mut buf, true)?;
        let tag = u32::from_be_bytes(buf);
        if tag == cache_mc_tag(b"FOR8") {
            self.mc_for8 = true;
        }
        Ok(tag)
    }

    fn cache_mc_read_u32(&mut self) -> Res<u32> {
        let mut buf = [0u8; 4];
        self.cache_read(&mut buf, false)?;
        let value = u32::from_be_bytes(buf);
        if self.mc_for8 {
            self.cache_read(&mut buf, false)?;
        }
        Ok(value)
    }

    fn cache_mc_read_u64(&mut self) -> Res<u64> {
        if !self.mc_for8 {
            Ok(self.cache_mc_read_u32()? as u64)
        } else {
            let mut buf = [0u8; 8];
            self.cache_read(&mut buf, false)?;
            Ok(u64::from_be_bytes(buf))
        }
    }

    fn cache_load_mc(&mut self) -> Res<()> {
        let mut count = 0u32;
        let mut time = 0u32;

        loop {
            let tag = self.cache_mc_read_tag()?;
            if tag == 0 {
                break;
            }

            if tag == cache_mc_tag(b"CACH") || tag == cache_mc_tag(b"MYCH") {
                continue;
            }
            if self.mc_for8 {
                let mut skip_buf = [0u8; 4];
                self.cache_read(&mut skip_buf, false)?;
            }

            let size = self.cache_mc_read_u64()?;
            let begin = self.file_offset;

            let alignment: u64 = if self.mc_for8 { 8 } else { 4 };

            let mut format = CacheDataFormat::Unknown;
            match &tag.to_be_bytes() {
                b"FOR4" => self.mc_for8 = false,
                b"FOR8" => self.mc_for8 = true,
                b"VRSN" | b"ETIM" => {
                    self.cache_mc_read_u32()?;
                }
                b"STIM" | b"TIME" => time = self.cache_mc_read_u32()?,
                b"CHNM" => {
                    check!(size > 0 && size < usize::MAX as u64);
                    let length = size as usize - 1;
                    let padded_length = (size as usize).wrapping_add(alignment as usize - 1) & !(alignment as usize - 1);
                    self.grow_array(padded_length)?;
                    let name = self.read_vec(padded_length)?;
                    check!(length <= padded_length);
                    self.channel_name = self.push_string_place_str(&name[..length])?;
                }
                b"SIZE" => count = self.cache_mc_read_u32()?,
                b"FVCA" => format = CacheDataFormat::Vec3Float,
                b"DVCA" => format = CacheDataFormat::Vec3Double,
                b"FBCA" => format = CacheDataFormat::RealFloat,
                b"DBCA" | b"DBLA" => format = CacheDataFormat::RealDouble,
                // "Unknown tag" is only on ufbx's error stack, not its description
                _ => return fail(),
            }

            if format != CacheDataFormat::Unknown {
                let elem_size = CACHE_DATA_FORMAT_SIZE[format as usize];
                let total_size = elem_size as u64 * count as u64;
                check!(size >= elem_size.wrapping_mul(count) as u64);

                self.frames.push(CacheFrame {
                    channel: self.channel_name.clone(),
                    time: time as f64 * (1.0 / 6000.0),
                    filename: self.stream_filename.clone(),
                    data_format: format,
                    data_encoding: CacheDataEncoding::BigEndian,
                    data_offset: self.file_offset,
                    data_count: count,
                    data_element_bytes: elem_size,
                    data_total_bytes: total_size,
                    file_format: CacheFileFormat::Mc,
                    ..Default::default()
                });

                let end = begin.wrapping_add(size.wrapping_add(alignment - 1) & !(alignment - 1));
                check!(end >= self.file_offset);
                let left = end - self.file_offset;
                self.cache_skip(left)?;
            }
        }

        Ok(())
    }

    fn cache_load_pc2(&mut self) -> Res<()> {
        let mut header = [0u8; 32];
        self.cache_read(&mut header, false)?;

        // `ufbxi_read_u32` in ufbx.c
        let u32_at = |i: usize| u32::from_le_bytes([header[i], header[i + 1], header[i + 2], header[i + 3]]);
        let num_points = u32_at(16);
        // `ufbxi_read_f32` in ufbx.c
        let start_frame = f32::from_bits(u32_at(20)) as f64;
        let frames_per_sample = f32::from_bits(u32_at(24)) as f64;
        let num_samples = u32_at(28);

        reserve(&mut self.frames, num_samples as usize, "temp")?;

        let total_points = num_points as u64 * num_samples as u64;
        check!(total_points < u64::MAX / 12);

        let mut offset = self.file_offset;

        // Skip almost to the end of the data and read one byte, as nothing follows the data to detect the end by
        if total_points > 0 {
            self.cache_skip(total_points * 12 - 1)?;
            let mut last_byte = [0u8; 1];
            self.cache_read(&mut last_byte, false)?;
        }

        for i in 0..num_samples {
            let sample_frame = start_frame + i as f64 * frames_per_sample;
            self.frames.push(CacheFrame {
                channel: self.channel_name.clone(),
                time: sample_frame / self.frames_per_second,
                filename: self.stream_filename.clone(),
                data_format: CacheDataFormat::Vec3Float,
                data_encoding: CacheDataEncoding::LittleEndian,
                data_offset: offset,
                data_count: num_points,
                data_element_bytes: 12,
                data_total_bytes: num_points.wrapping_mul(12) as u64,
                file_format: CacheFileFormat::Pc2,
                ..Default::default()
            });
            offset = offset.wrapping_add(num_points.wrapping_mul(12) as u64);
        }

        Ok(())
    }

    fn cache_load_xml_imp(&mut self, doc: &XmlTag) -> Res<()> {
        check!(!self.xml_loaded);

        self.xml_loaded = true;
        self.xml_ticks_per_frame = 250;
        self.xml_filename = self.stream_filename.clone();

        if let Some(tag_root) = doc.xml_find_child("Autodesk_Cache_File") {
            let tag_type = tag_root.xml_find_child("cacheType");
            let tag_fps = tag_root.xml_find_child("cacheTimePerFrame");
            let tag_channels = tag_root.xml_find_child("Channels");

            let mut extra_info = Vec::new();
            for tag in &tag_root.children {
                if tag.children.len() != 1 || tag.name != b"extra" {
                    continue;
                }
                extra_info.push(self.push_string_place_str(&tag.children[0].text)?);
            }
            self.cache.extra_info = extra_info;

            if let Some(tag_type) = tag_type {
                if let Some(t) = tag_type.xml_find_attrib("Type") {
                    match &t.value[..] {
                        b"OneFilePerFrame" => self.xml_type = CacheXmlType::FilePerFrame,
                        b"OneFile" => self.xml_type = CacheXmlType::SingleFile,
                        _ => {}
                    }
                }
                if let Some(format) = tag_type.xml_find_attrib("Format") {
                    match &format.value[..] {
                        b"mcc" => self.xml_format = CacheXmlFormat::Mcc,
                        b"mcx" => self.xml_format = CacheXmlFormat::Mcx,
                        _ => {}
                    }
                }
            }

            if let Some(fps) = tag_fps.and_then(|t| t.xml_find_attrib("TimePerFrame")) {
                let value = parse_uint32_radix(&fps.value, 10);
                if value > 0 {
                    self.xml_ticks_per_frame = value;
                }
            }

            if let Some(tag_channels) = tag_channels {
                for tag in &tag_channels.children {
                    let name = tag.xml_find_attrib("ChannelName");
                    let type_ = tag.xml_find_attrib("ChannelType");
                    let interpretation = tag.xml_find_attrib("ChannelInterpretation");
                    let (Some(name), Some(_), Some(interpretation)) = (name, type_, interpretation) else { continue };

                    let mut channel = TmpChannel {
                        name: self.push_string_place_str(&name.value)?,
                        interpretation: self.push_string_place_str(&interpretation.value)?,
                        ..Default::default()
                    };

                    let sampling_rate = tag.xml_find_attrib("SamplingRate");
                    let start_time = tag.xml_find_attrib("StartTime");
                    let end_time = tag.xml_find_attrib("EndTime");
                    if let (Some(sampling_rate), Some(start_time), Some(end_time)) = (sampling_rate, start_time, end_time) {
                        channel.sample_rate = parse_uint32_radix(&sampling_rate.value, 10);
                        channel.start_time = parse_uint32_radix(&start_time.value, 10);
                        channel.end_time = parse_uint32_radix(&end_time.value, 10);
                        channel.current_time = channel.start_time;
                        channel.try_load = true;
                    }
                    self.channels.push(channel);
                }
            }
        }

        // `ufbxi_tmp_channel_less` and `ufbxi_cache_sort_tmp_channels` in ufbx.c
        crate::search::stable_sort(&mut self.channels, 16, |a, b| a.name < b.name);
        Ok(())
    }

    fn cache_load_xml(&mut self) -> Res<()> {
        let mut data = self.buffer[self.pos..self.pos_end].to_vec();
        let start = self.stream.pos.min(self.stream.data.len() as u64) as usize;
        data.extend_from_slice(&self.stream.data[start..]);
        let doc = load_xml(&data)?;
        self.cache_load_xml_imp(&doc)
    }

    fn cache_load_file(&mut self, filename: &str) -> Res<()> {
        self.stream_filename = self.push_string_place_str(filename.as_bytes())?;

        // Assume all files have at least 16 bytes of header
        let magic_len = self.stream.read(&mut self.buffer[..16]);
        check!(magic_len == 16, "Truncated file");
        self.pos = 0;
        self.pos_end = 16;

        self.file_offset = 0;

        if &self.buffer[..11] == b"POINTCACHE2" {
            self.cache_load_pc2()
        } else if &self.buffer[..4] == b"FOR4" || &self.buffer[..4] == b"FOR8" {
            self.cache_load_mc()
        } else {
            self.cache_load_xml()
        }
    }

    fn cache_try_open_file(&mut self, filename: &str) -> Res<bool> {
        let Some(data) = open_file(self.opts.open_file_cb.as_ref(), filename, None, OpenFileType::GeometryCache) else { return Ok(false) };
        self.stream = Stream { data, pos: 0 };
        self.cache_load_file(filename)?;
        Ok(true)
    }

    fn cache_load_frame_files(&mut self) -> Res<()> {
        if self.xml_filename.is_empty() {
            return Ok(());
        }

        let extension = match self.xml_format {
            CacheXmlFormat::Mcc => "mc",
            CacheXmlFormat::Mcx => "mcx",
            CacheXmlFormat::None => return Ok(()),
        };

        let xml = self.xml_filename.as_bytes();
        let mut prefix_len = xml.len();
        for i in (1..=xml.len()).rev() {
            if xml[i - 1] == b'.' {
                prefix_len = i - 1;
                break;
            }
        }
        let prefix = self.xml_filename[..prefix_len].to_string();

        if self.xml_type == CacheXmlType::SingleFile {
            let filename = format!("{prefix}.{extension}");
            self.cache_try_open_file(&filename)?;
        } else if self.xml_type == CacheXmlType::FilePerFrame {
            let ticks_per_frame = self.xml_ticks_per_frame;
            let mut lowest_time: u32 = 0;
            loop {
                let mut time = u32::MAX;
                for chan in &mut self.channels {
                    if !chan.try_load || chan.consecutive_fails > 10 {
                        continue;
                    }
                    let sample_rate = if chan.sample_rate != 0 { chan.sample_rate } else { ticks_per_frame };
                    if chan.current_time < lowest_time {
                        let delta = (lowest_time - chan.current_time - 1) / sample_rate;
                        chan.current_time += delta * sample_rate;
                        if u32::MAX - chan.current_time >= sample_rate {
                            chan.current_time += sample_rate;
                        } else {
                            chan.try_load = false;
                            continue;
                        }
                    }
                    if chan.current_time <= chan.end_time {
                        time = time.min(chan.current_time);
                    }
                }
                if time == u32::MAX {
                    break;
                }

                let frame = time / ticks_per_frame;
                let tick = time % ticks_per_frame;
                let filename = if tick == 0 { format!("{prefix}Frame{frame}.{extension}") } else { format!("{prefix}Frame{frame}Tick{tick}.{extension}") };
                let found = self.cache_try_open_file(&filename)?;

                for chan in &mut self.channels {
                    if chan.current_time == time {
                        chan.consecutive_fails = if found { 0 } else { chan.consecutive_fails.wrapping_add(1) };
                    }
                }

                lowest_time = time + 1;
            }
        }

        Ok(())
    }

    fn cache_setup_channels(&mut self) {
        let frames = &mut self.cache.frames;
        let mut tmp_ix = 0;
        let mut channels = Vec::new();

        let mut begin = 0;
        while begin < frames.len() {
            let mut end = begin + 1;
            while end < frames.len() && frames[end].channel == frames[begin].channel {
                end += 1;
            }

            let mut chan = CacheChannel { name: frames[begin].channel.clone(), ..Default::default() };

            while tmp_ix < self.channels.len() && self.channels[tmp_ix].name < chan.name {
                tmp_ix += 1;
            }
            if tmp_ix < self.channels.len() && self.channels[tmp_ix].name == chan.name {
                chan.interpretation_name = self.channels[tmp_ix].interpretation.clone();
            }

            if frames[begin].file_format == CacheFileFormat::Pc2 {
                chan.interpretation = CacheInterpretation::VertexPosition;
            } else {
                for (interpretation, pattern) in CACHE_INTERPRETATION_NAMES {
                    if match_pattern(chan.interpretation_name.as_bytes(), pattern) {
                        chan.interpretation = interpretation;
                        break;
                    }
                }
            }

            let mut mirror_axis = MirrorAxis::None;
            let mut scale_factor: Real = 1.0;
            if chan.interpretation != CacheInterpretation::Unknown {
                mirror_axis = self.opts.mirror_axis;
                if self.opts.use_scale_factor {
                    scale_factor = self.opts.scale_factor;
                }
            }
            chan.mirror_axis = mirror_axis;
            chan.scale_factor = scale_factor;
            for f in &mut frames[begin..end] {
                f.mirror_axis = mirror_axis;
                f.scale_factor = scale_factor;
            }
            chan.frames = frames[begin..end].to_vec();

            channels.push(chan);
            begin = end;
        }

        self.cache.channels = channels;
    }

    fn cache_load_imp(&mut self, filename: &str) -> Res<()> {
        self.channel_name = String::new();

        if !self.cache_try_open_file(filename)? {
            return Err(Error::new("File not found").set_err_info(filename));
        }

        self.cache.root_filename = self.stream_filename.clone();

        self.cache_load_frame_files()?;

        self.cache.frames = std::mem::take(&mut self.frames);
        // `ufbxi_cmp_cache_frame_less` and `ufbxi_cache_sort_frames` in ufbx.c
        crate::search::stable_sort(&mut self.cache.frames, 16, |a, b| if a.channel != b.channel { a.channel < b.channel } else { a.time < b.time });
        self.cache_setup_channels();

        Ok(())
    }

    fn cache_load(&mut self, filename: &str) -> Res<GeometryCache> {
        match self.cache_load_imp(filename) {
            Ok(()) => Ok(std::mem::take(&mut self.cache)),
            Err(e) => Err(e.fix_error_type("Failed to load geometry cache")),
        }
    }
}

pub fn load_geometry_cache(filename: &str, opts: &GeometryCacheOpts) -> Res<GeometryCache> {
    let frames_per_second = if opts.frames_per_second > 0.0 { opts.frames_per_second } else { 30.0 };
    let mut warnings = Warnings::default();
    CacheContext::new(opts, frames_per_second, UnicodeErrorHandling::ReplacementCharacter, &mut warnings).cache_load(filename)
}

pub fn read_geometry_cache_real(frame: &CacheFrame, data: &mut [Real], opts: &GeometryCacheDataOpts) -> usize {
    let count = data.len();
    if count == 0 {
        return 0;
    }

    let (mut src_count, use_double) = match frame.data_format {
        CacheDataFormat::Unknown => (0, false),
        CacheDataFormat::RealFloat => (frame.data_count as usize, false),
        CacheDataFormat::Vec3Float => (frame.data_count as usize * 3, false),
        CacheDataFormat::RealDouble => (frame.data_count as usize, true),
        CacheDataFormat::Vec3Double => (frame.data_count as usize * 3, true),
    };

    let src_big_endian = match frame.data_encoding {
        CacheDataEncoding::Unknown => return 0,
        CacheDataEncoding::LittleEndian => false,
        CacheDataEncoding::BigEndian => true,
    };

    if src_count == 0 {
        return 0;
    }
    src_count = src_count.min(count);

    let Some(file) = open_file(opts.open_file_cb.as_ref(), &frame.filename, None, OpenFileType::GeometryCache) else { return 0 };
    let mut stream = Stream { data: file, pos: frame.data_offset };

    let mut dst = 0;
    let mut mirror_ix = (frame.mirror_axis as usize).wrapping_sub(1);
    let mut src = [0u8; GEOMETRY_CACHE_BUFFER_SIZE * 8];
    let mut buffer = [0.0 as Real; GEOMETRY_CACHE_BUFFER_SIZE];
    while src_count > 0 {
        let to_read = src_count.min(GEOMETRY_CACHE_BUFFER_SIZE);
        src_count -= to_read;
        let num_read;
        if use_double {
            let bytes_read = stream.read(&mut src[..to_read * 8]);
            num_read = bytes_read / 8;
            for (d, &b) in buffer.iter_mut().zip(src.as_chunks::<8>().0).take(num_read) {
                *d = if src_big_endian { f64::from_be_bytes(b) } else { f64::from_le_bytes(b) };
            }
        } else {
            let bytes_read = stream.read(&mut src[..to_read * 4]);
            num_read = bytes_read / 4;
            for (d, &b) in buffer.iter_mut().zip(src.as_chunks::<4>().0).take(num_read) {
                *d = (if src_big_endian { f32::from_be_bytes(b) } else { f32::from_le_bytes(b) }) as Real;
            }
        }

        if !opts.ignore_transform {
            let scale = frame.scale_factor;
            if scale != 1.0 {
                for v in &mut buffer[..num_read] {
                    *v *= scale;
                }
            }
            if frame.mirror_axis != MirrorAxis::None {
                while mirror_ix < num_read {
                    buffer[mirror_ix] = -buffer[mirror_ix];
                    mirror_ix += 3;
                }
                mirror_ix -= num_read;
            }
        }

        let weight: Real = if opts.use_weight { opts.weight } else { 1.0 };
        if opts.additive {
            for (d, &v) in data[dst..dst + num_read].iter_mut().zip(&buffer[..num_read]) {
                *d += v * weight;
            }
        } else {
            for (d, &v) in data[dst..dst + num_read].iter_mut().zip(&buffer[..num_read]) {
                *d = v * weight;
            }
        }
        dst += num_read;

        if num_read != to_read {
            break;
        }
    }

    dst
}

pub fn sample_geometry_cache_real(channel: &CacheChannel, time: f64, data: &mut [Real], opts: &GeometryCacheDataOpts) -> usize {
    let count = data.len();
    if count == 0 || channel.frames.is_empty() {
        return 0;
    }

    let mut opts = opts.clone();

    let frames = &channel.frames;
    let mut begin = 0;
    let mut end = frames.len();
    while end - begin >= 8 {
        let mid = (begin + end) >> 1;
        if frames[mid].time < time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    let eps = 0.00000001;

    end = frames.len();
    while begin < end {
        let next = &frames[begin];
        if next.time < time {
            begin += 1;
            continue;
        }

        if begin == 0 {
            return read_geometry_cache_real(next, data, &opts);
        }

        let prev = &frames[begin - 1];

        if (next.time - time).abs() < eps {
            return read_geometry_cache_real(next, data, &opts);
        }
        if (prev.time - time).abs() < eps {
            return read_geometry_cache_real(prev, data, &opts);
        }

        let rcp_delta = 1.0 / (next.time - prev.time);
        let t = (time - prev.time) * rcp_delta;

        let original_weight: Real = if opts.use_weight { opts.weight } else { 1.0 };

        opts.use_weight = true;
        opts.weight = original_weight * (1.0 - t);
        let num_prev = read_geometry_cache_real(prev, data, &opts);

        opts.additive = true;
        opts.weight = original_weight * t;
        return read_geometry_cache_real(next, &mut data[..num_prev], &opts);
    }

    read_geometry_cache_real(&frames[end - 1], data, &opts)
}

fn with_reals(data: &mut [Vec3], f: impl FnOnce(&mut [Real]) -> usize) -> usize {
    let mut reals: Vec<Real> = data.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
    let n = f(&mut reals);
    for (v, &[x, y, z]) in data.iter_mut().zip(reals.as_chunks::<3>().0) {
        *v = Vec3 { x, y, z };
    }
    n / 3
}

pub fn read_geometry_cache_vec3(frame: &CacheFrame, data: &mut [Vec3], opts: &GeometryCacheDataOpts) -> usize {
    if data.is_empty() {
        return 0;
    }
    with_reals(data, |reals| read_geometry_cache_real(frame, reals, opts))
}

pub fn sample_geometry_cache_vec3(channel: &CacheChannel, time: f64, data: &mut [Vec3], opts: &GeometryCacheDataOpts) -> usize {
    if data.is_empty() {
        return 0;
    }
    with_reals(data, |reals| sample_geometry_cache_real(channel, time, reals, opts))
}

/// Part of `ufbxi_load_external_cache` in ufbx.c
pub(crate) fn load_scene_cache(
    filename: &str,
    opts: &GeometryCacheOpts,
    fps: f64,
    handling: UnicodeErrorHandling,
    warnings: &mut Warnings,
) -> Res<GeometryCache> {
    CacheContext::new(opts, fps, handling, warnings).cache_load(filename)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::error::{ErrorType, WarningType};
    use crate::external::load_external_files_imp;
    use std::collections::HashMap;
    use std::sync::Mutex;

    type Files = HashMap<String, Vec<u8>>;

    fn memory_files(files: Files) -> (Arc<OpenFileFn>, Arc<Mutex<Vec<String>>>) {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let log = asked.clone();
        let cb: Arc<OpenFileFn> = Arc::new(move |path: &str, info: &OpenFileInfo| {
            assert_eq!(info.file_type, OpenFileType::GeometryCache);
            log.lock().unwrap().push(path.to_string());
            files.get(path).cloned()
        });
        (cb, asked)
    }

    fn opts(cb: &Arc<OpenFileFn>) -> GeometryCacheOpts {
        GeometryCacheOpts { open_file_cb: Some(cb.clone()), ..Default::default() }
    }

    fn data_opts(cb: &Arc<OpenFileFn>) -> GeometryCacheDataOpts {
        GeometryCacheDataOpts { open_file_cb: Some(cb.clone()), ..Default::default() }
    }

    fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        out
    }

    fn group(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = kind.iter().copied().chain(chunks.concat()).collect();
        chunk(b"FOR4", &body)
    }

    fn floats_be(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_be_bytes()).collect()
    }

    /// Ticks and each channel's points
    type McFrame<'a> = (u32, &'a [(&'a str, &'a [f32])]);

    fn mc_file(frames: &[McFrame]) -> Vec<u8> {
        let mut out =
            group(b"CACH", &[chunk(b"VRSN", b"0.1\0"), chunk(b"STIM", &frames[0].0.to_be_bytes()), chunk(b"ETIM", &frames[frames.len() - 1].0.to_be_bytes())]);
        for (time, channels) in frames {
            let mut chunks = vec![chunk(b"TIME", &time.to_be_bytes())];
            for (name, points) in channels.iter() {
                let mut n = name.as_bytes().to_vec();
                n.push(0);
                chunks.push(chunk(b"CHNM", &n));
                chunks.push(chunk(b"SIZE", &((points.len() / 3) as u32).to_be_bytes()));
                chunks.push(chunk(b"FVCA", &floats_be(points)));
            }
            out.extend(group(b"MYCH", &chunks));
        }
        out
    }

    fn description(kind: &str, format: &str, channels: &str) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\"?>\n<Autodesk_Cache_File>\n  <cacheType Type=\"{kind}\" Format=\"{format}\"/>\n  <cacheTimePerFrame TimePerFrame=\"250\"/>\n  <extra>made by hand</extra>\n  <extra> <x/> </extra>\n  <Channels>\n{channels}  </Channels>\n</Autodesk_Cache_File>\n"
        )
        .into_bytes()
    }

    fn channel_xml(i: usize, name: &str, interpretation: &str, rate: u32, start: u32, end: u32) -> String {
        format!(
            "    <channel{i} ChannelName=\"{name}\" ChannelType=\"FloatVectorArray\" ChannelInterpretation=\"{interpretation}\" SamplingType=\"Regular\" SamplingRate=\"{rate}\" StartTime=\"{start}\" EndTime=\"{end}\"/>\n"
        )
    }

    fn vec3s(v: &[Vec3]) -> Vec<Real> {
        v.iter().flat_map(|v| [v.x, v.y, v.z]).collect()
    }

    /// 12 points, enough that ufbx's 128-byte buffer never holds a file's end
    fn points(f: impl Fn(f32) -> f32) -> Vec<f32> {
        (0..36).map(|i| f(i as f32)).collect()
    }

    fn reals(v: &[f32]) -> Vec<Real> {
        v.iter().map(|&f| f as Real).collect()
    }

    fn pc2_file(num_points: u32, num_samples: u32) -> Vec<u8> {
        let mut pc2 = b"POINTCACHE2\0".to_vec();
        for v in [1u32, num_points] {
            pc2.extend_from_slice(&v.to_le_bytes());
        }
        pc2.extend_from_slice(&1.0f32.to_le_bytes());
        pc2.extend_from_slice(&0.5f32.to_le_bytes());
        pc2.extend_from_slice(&num_samples.to_le_bytes());
        for i in 0..num_points * 3 * num_samples {
            pc2.extend_from_slice(&(i as f32).to_le_bytes());
        }
        pc2
    }

    #[test]
    fn a_description_and_its_mc_file_give_sorted_channels_and_their_values() {
        let channels = channel_xml(0, "b", "normals", 250, 250, 500) + &channel_xml(1, "a", "Positions", 250, 250, 500);
        let (a1, a2) = (points(|i| i), points(|i| (36.0 - i) * 0.5));
        let b1: &[f32] = &[0.0, 0.0, 1.0];
        let mc = mc_file(&[(500, &[("a", &a2)]), (250, &[("b", b1), ("a", &a1)])]);
        let files: Files = [("d/c.xml".to_string(), description("OneFile", "mcc", &channels)), ("d/c.mc".to_string(), mc)].into();
        let (cb, asked) = memory_files(files);

        let cache = load_geometry_cache("d/c.xml", &opts(&cb)).unwrap();
        assert_eq!(*asked.lock().unwrap(), ["d/c.xml", "d/c.mc"]);
        assert_eq!(cache.root_filename, "d/c.xml");
        assert_eq!(cache.extra_info, ["made by hand"]);

        let names: Vec<(&str, f64)> = cache.frames.iter().map(|f| (f.channel.as_str(), f.time)).collect();
        assert_eq!(names, [("a", 250.0 / 6000.0), ("a", 500.0 / 6000.0), ("b", 250.0 / 6000.0)]);
        assert_eq!(cache.channels.len(), 2);
        let (a, b) = (&cache.channels[0], &cache.channels[1]);
        assert_eq!((a.name.as_str(), a.interpretation, a.interpretation_name.as_str()), ("a", CacheInterpretation::VertexPosition, "Positions"));
        assert_eq!((b.name.as_str(), b.interpretation, b.frames.len()), ("b", CacheInterpretation::VertexNormal, 1));
        assert!(a.frames.iter().all(|f| f.filename == "d/c.mc" && f.data_format == CacheDataFormat::Vec3Float && f.data_count == 12));

        let mut v = [Vec3::default(); 13];
        assert_eq!(read_geometry_cache_vec3(&a.frames[0], &mut v, &data_opts(&cb)), 12);
        assert_eq!(vec3s(&v[..12]), reals(&a1));
        assert_eq!(v[12], Vec3::default());

        let mut r = [0.0; 6];
        let t = (250.0 + 0.25 * 250.0) / 6000.0;
        assert_eq!(sample_geometry_cache_real(a, t, &mut r, &data_opts(&cb)), 6);
        let rcp_delta = 1.0 / (a.frames[1].time - a.frames[0].time);
        let w = (t - a.frames[0].time) * rcp_delta;
        let expected: Vec<Real> = a1.iter().zip(&a2).take(6).map(|(&p, &n)| p as Real * (1.0 - w) + n as Real * w).collect();
        assert_eq!(r.to_vec(), expected);
        assert_eq!(sample_geometry_cache_real(a, 1.0, &mut r, &data_opts(&cb)), 6);
        assert_eq!(r.to_vec(), reals(&a2[..6]));

        let flipped = GeometryCacheOpts { mirror_axis: MirrorAxis::X, use_scale_factor: true, scale_factor: 2.0, ..opts(&cb) };
        let cache = load_geometry_cache("d/c.xml", &flipped).unwrap();
        let mut r = [10.0; 7];
        assert_eq!(read_geometry_cache_real(&cache.channels[0].frames[0], &mut r, &GeometryCacheDataOpts { additive: true, ..data_opts(&cb) }), 7);
        assert_eq!(r, [10.0, 12.0, 14.0, 4.0, 18.0, 20.0, -2.0]);
        let raw = GeometryCacheDataOpts { ignore_transform: true, use_weight: true, weight: 0.5, ..data_opts(&cb) };
        assert_eq!(read_geometry_cache_real(&cache.channels[0].frames[0], &mut r[..4], &raw), 4);
        assert_eq!(r[..4], [0.0, 0.5, 1.0, 1.5]);
    }

    #[test]
    fn frame_files_are_tried_frame_by_frame_and_tick_by_tick() {
        let channels = channel_xml(0, "a", "points", 125, 250, 750);
        let point: &[f32] = &[1.0, 1.0, 1.0];
        let files: Files = [
            ("c.xml".to_string(), description("OneFilePerFrame", "mcx", &channels)),
            ("cFrame1.mcx".to_string(), mc_file(&[(250, &[("a", point)])])),
            ("cFrame2.mcx".to_string(), mc_file(&[(500, &[("a", point)])])),
        ]
        .into();
        let (cb, asked) = memory_files(files);
        let cache = load_geometry_cache("c.xml", &opts(&cb)).unwrap();
        assert_eq!(*asked.lock().unwrap(), ["c.xml", "cFrame1.mcx", "cFrame1Tick125.mcx", "cFrame2.mcx", "cFrame2Tick125.mcx", "cFrame3.mcx"]);
        assert_eq!(cache.channels.len(), 1);
        assert_eq!(cache.channels[0].interpretation, CacheInterpretation::Points);
        assert_eq!(cache.channels[0].frames.iter().map(|f| f.filename.as_str()).collect::<Vec<_>>(), ["cFrame1.mcx", "cFrame2.mcx"]);
    }

    #[test]
    fn pc2_frames_are_spaced_by_their_sample_rate() {
        let pc2 = pc2_file(20, 3);
        let truncated = pc2[..pc2.len() - 1].to_vec();
        let (cb, _) = memory_files([("box.pc2".to_string(), pc2), ("cut.pc2".to_string(), truncated)].into());

        let cache = load_geometry_cache("box.pc2", &GeometryCacheOpts { frames_per_second: 24.0, ..opts(&cb) }).unwrap();
        let times: Vec<f64> = cache.frames.iter().map(|f| f.time).collect();
        assert_eq!(times, [1.0 / 24.0, 1.5 / 24.0, 2.0 / 24.0]);
        let chan = &cache.channels[0];
        assert_eq!((chan.name.as_str(), chan.interpretation, chan.frames.len()), ("", CacheInterpretation::VertexPosition, 3));
        assert_eq!((chan.frames[1].data_offset, chan.frames[1].data_encoding), (32 + 240, CacheDataEncoding::LittleEndian));
        let mut v = [Vec3::default(); 2];
        assert_eq!(read_geometry_cache_vec3(&chan.frames[2], &mut v, &data_opts(&cb)), 2);
        assert_eq!(vec3s(&v), [120.0, 121.0, 122.0, 123.0, 124.0, 125.0]);

        assert_eq!(load_geometry_cache("cut.pc2", &opts(&cb)).unwrap_err().description, "Truncated file");
    }

    #[test]
    fn a_short_last_read_serves_stale_buffer_bytes_as_in_ufbx() {
        // Past a short refill of its 128-byte buffer ufbx reads the bytes left from earlier reads: here an MC file's
        // end reads as the tag `-2.0f` of an earlier frame, and a cut inside a small point cache goes unnoticed
        let a2: &[f32] = &[3.0, 2.0, 1.0, 0.0, -1.0, -2.0];
        let mc = mc_file(&[(500, &[("a", a2)]), (250, &[("b", &[0.0, 0.0, 1.0]), ("a", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])])]);
        let pc2 = pc2_file(2, 3);
        let (cb, _) = memory_files([("c.mc".to_string(), mc), ("cut.pc2".to_string(), pc2[..pc2.len() - 1].to_vec())].into());
        assert_eq!(load_geometry_cache("c.mc", &opts(&cb)).unwrap_err().description, "Failed to load geometry cache");
        assert_eq!(load_geometry_cache("cut.pc2", &opts(&cb)).unwrap().frames.len(), 3);
    }

    #[test]
    fn failures_carry_ufbx_descriptions() {
        let (cb, _) = memory_files(
            [
                ("short.mc".to_string(), b"FOR4\0\0\0\x04CACH".to_vec()),
                ("odd.mc".to_string(), group(b"CACH", &[chunk(b"WHAT", &[0; 4])])),
                ("twice.xml".to_string(), description("OneFilePerFrame", "mcc", &channel_xml(0, "a", "points", 250, 250, 250))),
                ("twiceFrame1.mc".to_string(), description("OneFile", "mcc", "")),
                ("deep.xml".to_string(), "<a>".repeat(40).into_bytes()),
            ]
            .into(),
        );
        let err = load_geometry_cache("none.xml", &opts(&cb)).unwrap_err();
        assert_eq!((err.kind, err.description.as_str(), err.info.as_str()), (ErrorType::FileNotFound, "File not found", "none.xml"));
        let err = load_geometry_cache("none.xml", &GeometryCacheOpts::default()).unwrap_err();
        assert_eq!(err.kind, ErrorType::FileNotFound);

        assert_eq!(load_geometry_cache("short.mc", &opts(&cb)).unwrap_err().description, "Truncated file");
        for name in ["odd.mc", "twice.xml", "deep.xml"] {
            let err = load_geometry_cache(name, &opts(&cb)).unwrap_err();
            assert_eq!((err.kind, err.description.as_str()), (ErrorType::Unknown, "Failed to load geometry cache"), "{name}");
        }
    }

    #[test]
    fn external_files_load_once_and_reach_their_deformers() {
        let channels = channel_xml(0, "a", "positions", 250, 250, 250) + &channel_xml(1, "b", "positions", 250, 250, 250);
        let p = points(|i| i);
        let files: Files =
            [("/abs/c.xml".to_string(), description("OneFile", "mcc", &channels)), ("/abs/c.mc".to_string(), mc_file(&[(250, &[("a", &p), ("b", &p)])]))]
                .into();
        let (cb, asked) = memory_files(files);

        let mut scene = Scene::default();
        scene.settings.frames_per_second = 24.0;
        scene.metadata.geometry_scale = 0.01;
        let file = |name: &str, abs: &str| CacheFile { filename: name.to_string(), absolute_filename: abs.to_string(), ..Default::default() };
        scene.cache_files = vec![file("c.xml", "/abs/c.xml"), file("missing.xml", "/abs/missing.xml"), file("c.xml", "/abs/c.xml"), file("", "")];
        let deformer = |f: usize, channel: &str| CacheDeformer { file: Some(f), channel: channel.to_string(), ..Default::default() };
        scene.cache_deformers = vec![deformer(0, "b"), deformer(2, "zz"), deformer(1, "a"), deformer(3, "a")];
        let base = scene.clone();

        let opts = LoadOpts { open_file_cb: Some(cb.clone()), ignore_missing_external_files: true, ..Default::default() };
        let mut warnings = Warnings::default();
        load_external_files_imp(&mut scene, &opts, MirrorAxis::Z, &mut warnings).unwrap();
        assert_eq!(*asked.lock().unwrap(), ["c.xml", "/abs/c.xml", "/abs/c.mc", "missing.xml", "/abs/missing.xml"]);
        assert_eq!(warnings.list.len(), 1);
        assert_eq!(
            (warnings.list[0].kind, warnings.list[0].description.as_str()),
            (WarningType::MissingExternalFile, "Failed to open geometry cache: missing.xml")
        );

        let cache = scene.cache_files[0].external_cache.clone().unwrap();
        assert!(Arc::ptr_eq(&cache, scene.cache_files[2].external_cache.as_ref().unwrap()));
        assert!(scene.cache_files[1].external_cache.is_none() && scene.cache_files[3].external_cache.is_none());
        assert_eq!((cache.channels[1].mirror_axis, cache.channels[1].scale_factor), (MirrorAxis::Z, 0.01));
        let linked: Vec<(bool, Option<usize>)> = scene.cache_deformers.iter().map(|d| (d.external_cache.is_some(), d.external_channel)).collect();
        assert_eq!(linked, [(true, Some(1)), (true, None), (false, None), (false, None)]);

        let mut scene = base;
        let strict = LoadOpts { ignore_missing_external_files: false, ..opts };
        let err = load_external_files_imp(&mut scene, &strict, MirrorAxis::None, &mut Warnings::default()).unwrap_err();
        assert_eq!((err.kind, err.description.as_str(), err.info.as_str()), (ErrorType::ExternalFileNotFound, "External file not found", "/abs/missing.xml"));
    }

    #[test]
    fn the_merge_sort_is_stable_and_takes_nan_times() {
        let mut seed = 0x2545f491u64;
        for size in 0..80 {
            let mut v: Vec<(u64, usize)> = (0..size)
                .map(|i| {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    (seed % 7, i)
                })
                .collect();
            let mut expected = v.clone();
            expected.sort_by_key(|x| x.0);
            crate::search::stable_sort(&mut v, 16, |a, b| a.0 < b.0);
            assert_eq!(v, expected);
        }
        let mut times = vec![3.0, f64::NAN, 1.0, f64::INFINITY, 2.0, f64::NAN, 0.0];
        crate::search::stable_sort(&mut times, 16, |a, b| a < b);
        assert_eq!(times.len(), 7);
    }

    fn corpus(rel: &str) -> Option<String> {
        let root = std::env::var("UFBX_DATA").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../ext/ufbx/data").to_string());
        let path = format!("{root}/{rel}");
        if std::path::Path::new(&path).exists() {
            Some(path)
        } else {
            eprintln!("skipped: no {path}");
            None
        }
    }

    fn disk() -> GeometryCacheOpts {
        GeometryCacheOpts { open_file_cb: Some(Arc::new(crate::paths::default_open_file)), ..Default::default() }
    }

    fn disk_data() -> GeometryCacheDataOpts {
        GeometryCacheDataOpts { open_file_cb: Some(Arc::new(crate::paths::default_open_file)), ..Default::default() }
    }

    #[test]
    fn the_corpus_sine_caches_follow_their_sine() {
        for (rel, begin, end, threshold) in [
            ("caches/sine_mcmf_undersample/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.04),
            ("caches/sine_mcsd_oversample/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.003),
            ("caches/sine_mxmd_oversample/cache.xml", 11.0 / 24.0, 19.0 / 24.0, 0.003),
            ("caches/sine_mxsf_regular/cache.xml", 1.0 / 24.0, 29.0 / 24.0, 0.008),
        ] {
            let Some(path) = corpus(rel) else { return };
            let cache = load_geometry_cache(&path, &disk()).unwrap();
            assert_eq!(cache.channels.len(), 2);
            assert!(cache.channels.iter().all(|c| c.interpretation == CacheInterpretation::VertexPosition));
            let chan = cache.channels.iter().find(|c| c.name == "pCubeShape1").unwrap();
            let mut time: f64 = begin;
            while time <= end + 0.0001 {
                let mut pos = [Vec3::default(); 64];
                let n = sample_geometry_cache_vec3(chan, time, &mut pos, &disk_data());
                assert_eq!(n, 36, "{rel} at {time}");
                let t = (time - 1.0 / 24.0) / (29.0 / 24.0) * 4.0;
                for v in &pos[..n] {
                    let sx = ((v.y + t * 0.5) * std::f64::consts::PI * 2.0).sin() * 0.25;
                    let vx = v.x + if v.x > 0.0 { -0.5 } else { 0.5 };
                    assert!((vx - sx).abs() < threshold, "{rel} at {time}: {vx} vs {sx}");
                }
                time += 0.1 / 24.0;
            }
        }
    }

    #[test]
    fn the_corpus_xml_parse_case_decodes_entities_and_cdata() {
        let Some(path) = corpus("caches/sine_xml_parse/cache.xml") else { return };
        let cache = load_geometry_cache(&path, &disk()).unwrap();
        let ex0 = "cdata! \"'&<><--&lt;&#x61;<tag></tag>-->!CDATA[]>aβカ😂]]";
        let ex1 = "\"'&<>aaaβββカカカ😂😂😂";
        assert_eq!(cache.extra_info, [ex0, ex1]);
        let names: Vec<(&str, CacheInterpretation)> = cache.channels.iter().map(|c| (c.interpretation_name.as_str(), c.interpretation)).collect();
        assert_eq!(names, [("<!--\"positions\"-->", CacheInterpretation::Unknown), ("<![CDATA[<positions>]]>", CacheInterpretation::Unknown)]);

        let Some(path) = corpus("cache_xml_depth.xml") else { return };
        assert_eq!(load_geometry_cache(&path, &disk()).unwrap_err().kind, ErrorType::Unknown);
        let Some(path) = corpus("caches/indirect_xml_fail/cache.xml") else { return };
        assert!(load_geometry_cache(&path, &disk()).is_err());
    }

    #[test]
    fn the_corpus_point_cache_reads_every_vertex() {
        let Some(path) = corpus("max_cache_box_7500_binary_fpc/max_cache_box.pc2") else { return };
        let cache = load_geometry_cache(&path, &disk()).unwrap();
        assert_eq!(cache.frames.len(), 11);
        for frame in &cache.frames {
            assert_eq!((frame.file_format, frame.data_count), (CacheFileFormat::Pc2, 770));
            let mut v = vec![Vec3::default(); 770];
            assert_eq!(read_geometry_cache_vec3(frame, &mut v, &disk_data()), 770);
        }
    }
}
