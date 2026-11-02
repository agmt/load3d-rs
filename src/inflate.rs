use crate::types::{Progress, ProgressResult};

/// `ufbxi_deflate_length_lut` in ufbx.c: [0:5] extra bits, [5:8] flags, [16:32] base value
const LENGTH_LUT: [u32; 32] = [
    0x00000020, 0x00030040, 0x00040040, 0x00050040, 0x00060040, 0x00070040, 0x00080040, 0x00090040, 0x000a0040, 0x000b0041, 0x000d0041, 0x000f0041, 0x00110041,
    0x00130042, 0x00170042, 0x001b0042, 0x001f0042, 0x00230043, 0x002b0043, 0x00330043, 0x003b0043, 0x00430044, 0x00530044, 0x00630044, 0x00730044, 0x00830045,
    0x00a30045, 0x00c30045, 0x00e30045, 0x01020040, 0x00010020, 0x00010020,
];
/// `ufbxi_deflate_dist_lut` in ufbx.c
const DIST_LUT: [u32; 32] = [
    0x00010000, 0x00020000, 0x00030000, 0x00040000, 0x00050001, 0x00070001, 0x00090002, 0x000d0002, 0x00110003, 0x00190003, 0x00210004, 0x00310004, 0x00410005,
    0x00610005, 0x00810006, 0x00c10006, 0x01010007, 0x01810007, 0x02010008, 0x03010008, 0x04010009, 0x06010009, 0x0801000a, 0x0c01000a, 0x1001000b, 0x1801000b,
    0x2001000c, 0x3001000c, 0x4001000d, 0x6001000d, 0x00010020, 0x00010020,
];
const CODE_LENGTH_PERMUTATION: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

const FAST_MIN_IN: usize = 8;
const FAST_MIN_OUT: isize = 2;

const HUFF_MAX_BITS: usize = 16;
const HUFF_MAX_VALUE: usize = 288;
const HUFF_FAST_BITS: u32 = 10;
const HUFF_FAST_SIZE: usize = 1 << HUFF_FAST_BITS;
const HUFF_FAST_MASK: u64 = HUFF_FAST_SIZE as u64 - 1;
const HUFF_MAX_LONG_BITS: u32 = 5;
const HUFF_MAX_LONG_SYMS: usize = 380;
const HUFF_CODELEN_FAST_BITS: u32 = 8;
const HUFF_CODELEN_FAST_MASK: u32 = (1 << HUFF_CODELEN_FAST_BITS) - 1;
const HUFF_MAX_EXTRA_SYMS: usize = 32;
const HUFF_CODELEN_SYMS: usize = 19;
const HUFF_MAX_COMBINED_SYMS: usize = 320;

const SYM_END: u16 = 0x20;
const SYM_MATCH: u16 = 0x40;
const SYM_FAST: u16 = 0x80;
const ERROR_SYM: u16 = 0x0120;

const LOCAL_BUFFER_SIZE: usize = 256;

fn huff_sym_total_bits(sym: u16) -> u32 {
    sym as u32 & 0x1f
}

fn huff_sym_long_mask(sym: u16) -> u32 {
    sym as u32 & 0x1f
}

fn huff_sym_long_offset(sym: u16) -> u32 {
    sym as u32 >> 7
}

fn huff_sym_value(sym: u16) -> u32 {
    sym as u32 >> 8
}

type ReadFn<'a> = dyn FnMut(&mut [u8]) -> usize + 'a;

#[derive(Default)]
pub struct InflateInput<'a> {
    /// Bytes of compressed input in all, from `data` and then from `read_fn`
    pub total_size: usize,
    pub data: &'a [u8],
    /// The size of the buffer `read_fn` reads into, at least ufbx's 256 bytes
    pub buffer_size: usize,
    /// Fills the slice from the start, returning the bytes read: 0 at the end, more than asked (`usize::MAX`) for an error
    pub read_fn: Option<&'a mut ReadFn<'a>>,
    /// `ProgressFn`, but free to borrow
    pub progress_cb: Option<&'a (dyn Fn(&Progress) -> ProgressResult + Send + Sync + 'a)>,
    pub progress_interval_hint: u64,
    pub progress_size_before: u64,
    pub progress_size_after: u64,
    pub no_header: bool,
    pub no_checksum: bool,
    /// 0 for ufbx's choice, else the bits of the fast Huffman lookup: 1 to 8 or 10
    pub internal_fast_bits: usize,
}

#[derive(Clone)]
struct HuffTree {
    fast_sym: [u16; HUFF_FAST_SIZE],
    long_sym: [u16; HUFF_MAX_LONG_SYMS],
    sorted_to_sym: [u16; HUFF_MAX_VALUE],
    extra_shift_base: [u32; HUFF_MAX_EXTRA_SYMS],
    extra_mask: [u16; HUFF_MAX_EXTRA_SYMS],
    past_max_code: [u16; HUFF_MAX_BITS],
    code_to_sorted: [i16; HUFF_MAX_BITS],
}

impl HuffTree {
    const ZERO: HuffTree = HuffTree {
        fast_sym: [0; HUFF_FAST_SIZE],
        long_sym: [0; HUFF_MAX_LONG_SYMS],
        sorted_to_sym: [0; HUFF_MAX_VALUE],
        extra_shift_base: [0; HUFF_MAX_EXTRA_SYMS],
        extra_mask: [0; HUFF_MAX_EXTRA_SYMS],
        past_max_code: [0; HUFF_MAX_BITS],
        code_to_sorted: [0; HUFF_MAX_BITS],
    };
}

#[derive(Clone)]
struct Trees {
    lit_length: HuffTree,
    dist: HuffTree,
    fast_bits: u32,
}

impl Trees {
    const ZERO: Trees = Trees { lit_length: HuffTree::ZERO, dist: HuffTree::ZERO, fast_bits: 0 };
}

/// The fixed Huffman tables, built by the first call that needs them
#[derive(Default)]
pub struct InflateRetain {
    static_trees: Option<Box<Trees>>,
}

impl InflateRetain {
    pub fn new() -> InflateRetain {
        InflateRetain::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Chunk {
    Data,
    Buffer,
}

/// The chunk pointers are offsets into `data` or `buffer`, both of which begin the chunk
struct BitStream<'s> {
    input_left: usize,
    read_fn: Option<&'s mut ReadFn<'s>>,
    data: &'s [u8],
    buffer: Vec<u8>,
    buffer_size: usize,
    chunk: Chunk,
    chunk_ptr: usize,
    chunk_yield: usize,
    chunk_end: usize,
    chunk_real_end: usize,
    num_read_before_chunk: usize,
    progress_bias: u64,
    progress_total: u64,
    progress_interval: usize,
    bits: u64,
    left: usize,
    progress_cb: Option<&'s (dyn Fn(&Progress) -> ProgressResult + Send + Sync + 's)>,
    seen_end: bool,
    stop_error: i8,
}

// `ptr + progress_interval` wraps below `ptr` for intervals within 8 of `SIZE_MAX`, as C's pointer does
fn after_yield(ptr: usize, chunk_yield: usize) -> bool {
    (ptr.wrapping_sub(chunk_yield) as isize) > 0
}

fn read_u64(b: &[u8], pos: usize) -> u64 {
    if let Some(s) = b.get(pos..pos.wrapping_add(8)) {
        return u64::from_le_bytes(s.try_into().unwrap_or_default());
    }
    let mut v = 0u64;
    for i in 0..8 {
        if let Some(&x) = b.get(pos.wrapping_add(i)) {
            v |= (x as u64) << (8 * i);
        }
    }
    v
}

impl<'s> BitStream<'s> {
    fn bit_stream_init(input: &'s mut InflateInput) -> BitStream<'s> {
        let data_size = input.data.len().min(input.total_size);
        let input_left = input.total_size - data_size;
        let buffer_size = if input.buffer_size > LOCAL_BUFFER_SIZE { input.buffer_size } else { LOCAL_BUFFER_SIZE };
        // never more is read into the buffer than what is left of the input after under 64 carried-over bytes
        let buffer_alloc = buffer_size.min(input_left.saturating_add(64)).max(LOCAL_BUFFER_SIZE);
        let progress_cb = input.progress_cb;
        let progress_interval = if progress_cb.is_none() || input.progress_interval_hint >= usize::MAX as u64 {
            usize::MAX
        } else if input.progress_interval_hint > 0 {
            input.progress_interval_hint as usize
        } else {
            0x4000
        };
        let mut s = BitStream {
            input_left,
            data: &input.data[..data_size],
            read_fn: match &mut input.read_fn {
                Some(f) => Some(&mut **f),
                None => None,
            },
            buffer: vec![0; buffer_alloc],
            buffer_size,
            chunk: Chunk::Data,
            chunk_ptr: 0,
            chunk_yield: 0,
            chunk_end: data_size.max(8) - 8,
            chunk_real_end: data_size,
            num_read_before_chunk: 0,
            progress_bias: input.progress_size_before,
            progress_total: (input.total_size as u64).wrapping_add(input.progress_size_before).wrapping_add(input.progress_size_after),
            progress_interval,
            bits: 0,
            left: 0,
            progress_cb,
            seen_end: false,
            stop_error: 0,
        };
        if data_size < 64 {
            s.bit_chunk_refill(0);
        }
        s.chunk_yield = if s.progress_cb.is_some() && s.chunk_end.wrapping_sub(s.chunk_ptr) > s.progress_interval.wrapping_add(8) {
            s.chunk_ptr.wrapping_add(s.progress_interval)
        } else {
            s.chunk_end
        };
        s
    }

    fn chunk_bytes(&self) -> &[u8] {
        match self.chunk {
            Chunk::Data => self.data,
            Chunk::Buffer => &self.buffer,
        }
    }

    fn bit_chunk_refill(&mut self, ptr: usize) -> usize {
        let mut left = self.chunk_real_end - ptr;
        if left > 0 {
            match self.chunk {
                Chunk::Data => self.buffer[..left].copy_from_slice(&self.data[ptr..ptr + left]),
                Chunk::Buffer => self.buffer.copy_within(ptr..ptr + left, 0),
            }
        }
        self.num_read_before_chunk = self.num_read_before_chunk.wrapping_add(ptr);

        if self.stop_error == 0
            && let Some(read_fn) = self.read_fn.as_deref_mut()
        {
            let to_read = self.input_left.min(self.buffer_size - left);
            if to_read > 0 {
                let mut num_read = read_fn(&mut self.buffer[left..left + to_read]);
                if num_read > to_read {
                    num_read = 0;
                }
                self.input_left -= num_read;
                left += num_read;
            }
        }

        if left < 64 {
            if self.seen_end {
                self.stop_error = -31;
            }
            self.seen_end = true;
            self.buffer[left..128].fill(0);
            left = 128;
        }

        self.chunk = Chunk::Buffer;
        self.chunk_ptr = 0;
        self.chunk_end = left - 8;
        self.chunk_real_end = left;
        0
    }

    fn bit_yield(&mut self, mut ptr: usize) -> usize {
        if ptr > self.chunk_end {
            ptr = self.bit_chunk_refill(ptr);
        }

        if let Some(cb) = self.progress_cb {
            let num_read = self.num_read_before_chunk.wrapping_add(ptr);
            let progress = Progress { bytes_read: self.progress_bias.wrapping_add(num_read as u64), bytes_total: self.progress_total };
            if cb(&progress) == ProgressResult::Cancel {
                self.stop_error = -28;
                ptr = 0;
                self.buffer.truncate(LOCAL_BUFFER_SIZE);
                self.buffer.fill(0);
                self.buffer_size = LOCAL_BUFFER_SIZE;
                self.chunk = Chunk::Buffer;
                self.chunk_ptr = 0;
                self.chunk_end = LOCAL_BUFFER_SIZE - 8;
                self.chunk_real_end = LOCAL_BUFFER_SIZE;
            }
        }

        self.chunk_yield = if self.progress_cb.is_some() && self.chunk_end.wrapping_sub(ptr) > self.progress_interval.wrapping_add(8) {
            ptr.wrapping_add(self.progress_interval)
        } else {
            self.chunk_end
        };
        ptr
    }

    /// [56, 63] bits valid in `bits` after it
    #[inline(always)]
    fn bit_refill(&mut self, bits: &mut u64, left: &mut usize, data: &mut usize) {
        if after_yield(*data, self.chunk_yield) {
            *data = self.bit_yield(*data);
        }
        *bits |= read_u64(self.chunk_bytes(), *data).wrapping_shl(*left as u32);
        *data = data.wrapping_add(63usize.wrapping_sub(*left) >> 3);
        *left |= 56;
    }

    fn bit_copy_bytes(&mut self, dst: &mut [u8]) -> bool {
        let mut len = dst.len();
        let mut ptr = 0;
        while len > 0 && self.left > 0 {
            dst[ptr] = self.bits as u8;
            ptr += 1;
            len -= 1;
            self.bits >>= 8;
            self.left -= 8;
        }
        if len == 0 {
            return true;
        }

        // bits read ahead past `left` would be stale after the copy
        self.bits = 0;

        let chunk_left = self.chunk_real_end - self.chunk_ptr;
        let at = self.chunk_ptr;
        if chunk_left >= len {
            dst[ptr..ptr + len].copy_from_slice(&self.chunk_bytes()[at..at + len]);
            self.chunk_ptr += len;
            return true;
        } else {
            dst[ptr..ptr + chunk_left].copy_from_slice(&self.chunk_bytes()[at..at + chunk_left]);
            self.chunk_ptr += chunk_left;
            ptr += chunk_left;
            len -= chunk_left;
        }

        if len > self.input_left {
            return false;
        }
        let mut num_read = 0;
        if let Some(read_fn) = self.read_fn.as_deref_mut() {
            num_read = read_fn(&mut dst[ptr..ptr + len]);
            self.input_left = self.input_left.wrapping_sub(num_read);
        }
        num_read == len
    }
}

struct DeflateContext<'s> {
    stream: BitStream<'s>,
    fast_bits: u32,
    out_ptr: usize,
}

fn bit_reverse(mask: u32, num_bits: u32) -> u32 {
    let mut x = mask;
    x = ((x & 0xaaaa) >> 1) | ((x & 0x5555) << 1);
    x = ((x & 0xcccc) >> 2) | ((x & 0x3333) << 2);
    x = ((x & 0xf0f0) >> 4) | ((x & 0x0f0f) << 4);
    x = ((x & 0xff00) >> 8) | ((x & 0x00ff) << 8);
    x >> (16 - num_bits)
}

/// 0, -1 for an overfull code, -2 for an underfull one
fn huff_build_imp(tree: &mut HuffTree, sym_bits: &[u8], sym_extra: &[u32], sym_extra_offset: u32, fast_bits: u32, bits_counts: &[u32; HUFF_MAX_BITS]) -> isize {
    let fast_mask = (1u32 << fast_bits) - 1;
    let sym_count = sym_bits.len() as u32;
    let nonzero_sym_count = sym_count - bits_counts[0];

    let mut total_syms = [0u32; HUFF_MAX_BITS];
    let mut first_code = [0u32; HUFF_MAX_BITS];

    tree.code_to_sorted[0] = i16::MAX;
    tree.past_max_code[0] = 0;
    total_syms[0] = 0;

    let mut last_valid_prefix = 0u32;

    {
        let mut num_codes_left: i32 = 1;
        let mut code = 0u32;
        let mut prev_count = 0u32;
        let mut long_offset = 0u32;
        for bits in 1..HUFF_MAX_BITS {
            let count = bits_counts[bits];
            code = (code + prev_count) << 1;
            first_code[bits] = code;
            tree.past_max_code[bits] = (code + count) as u16;

            let prev_syms = total_syms[bits - 1];
            total_syms[bits] = prev_syms + count;

            num_codes_left = (num_codes_left << 1) - count as i32;
            if num_codes_left < 0 {
                return -1;
            }

            let bits = bits as u32;
            if count > 0 && bits > fast_bits && bits - fast_bits <= HUFF_MAX_LONG_BITS {
                let shift = bits - fast_bits;
                let last_inclusive = if num_codes_left == 0 { (1u32 << shift) - 1 } else { 0 };
                let first_prefix = code >> shift;
                let last_prefix = (code + count + last_inclusive) >> shift;
                let mask = (1u32 << shift) - 1;
                let half_step = 1u32 << (shift - 1);
                for prefix in first_prefix..last_prefix {
                    let rev_prefix = bit_reverse(prefix, fast_bits);
                    tree.fast_sym[rev_prefix as usize] = (mask | long_offset << 8) as u16;
                    long_offset += half_step;
                }
                last_valid_prefix = last_prefix;
            }

            tree.code_to_sorted[bits as usize] = if count > 0 { (prev_syms as i32 - code as i32) as i16 } else { i16::MAX };
            prev_count = count;
        }

        if (nonzero_sym_count > 1 && num_codes_left != 0) || (nonzero_sym_count == 1 && total_syms[1] != 1) {
            return -2;
        }
    }

    let mut num_extra = 0u32;
    tree.extra_shift_base[0] = 0;
    tree.extra_mask[0] = 0;

    // a full code (two or more symbols) covers every fast entry itself
    if nonzero_sym_count <= 1 {
        tree.fast_sym[..=fast_mask as usize].fill(ERROR_SYM);
    }

    let mut bits_index = [0u32; HUFF_MAX_BITS];
    for i in 0..sym_count {
        let bits = sym_bits[i as usize] as u32;
        if bits == 0 {
            continue;
        }

        let mut sym = i << 8 | bits;
        if i >= sym_extra_offset {
            let extra = sym_extra[(i - sym_extra_offset) as usize];
            sym = sym.wrapping_add(extra);

            if (extra & 0xffff001f) != 0 && (extra & 0x20) == 0 {
                num_extra += 1;
                let ix = num_extra;
                tree.extra_shift_base[ix as usize] = (extra & 0xffff0000) | bits;
                tree.extra_mask[ix as usize] = ((1u32 << (extra & 0x1f)) - 1) as u16;
                sym = (sym & 0xff) | ix << 8;
            }
        }

        let index = bits_index[bits as usize];
        bits_index[bits as usize] += 1;
        let sorted = total_syms[bits as usize - 1] + index;
        tree.sorted_to_sym[sorted as usize] = sym as u16;

        let code = first_code[bits as usize] + index;
        let rev_code = bit_reverse(code, bits);

        if bits <= fast_bits {
            let mut fast_sym = sym;
            if (fast_sym & SYM_END as u32) == 0 {
                fast_sym |= SYM_FAST as u32;
            }
            let hi_max = 1u32 << (fast_bits - bits);
            for hi in 0..hi_max {
                tree.fast_sym[(rev_code | hi << bits) as usize] = fast_sym as u16;
            }
        } else if bits <= fast_bits + HUFF_MAX_LONG_BITS && (code >> (bits - fast_bits)) < last_valid_prefix {
            let fast_sym = tree.fast_sym[(rev_code & fast_mask) as usize] as u32;
            let mut long_bits = 0u32;
            let mut long_mask = fast_sym;
            while long_bits < HUFF_MAX_LONG_BITS && (long_mask & 1) != 0 {
                long_mask >>= 1;
                long_bits += 1;
            }

            let long_base = fast_sym >> 7;
            let lo_bits = bits - fast_bits;
            let hi_max = 1u32 << (long_bits - lo_bits);
            let rev_suffix = rev_code >> fast_bits;
            for hi in 0..hi_max {
                tree.long_sym[(long_base + (rev_suffix | hi << lo_bits)) as usize] = sym as u16;
            }
        } else {
            let fast_sym = (code >> (bits - fast_bits)) << 8;
            tree.fast_sym[(rev_code & fast_mask) as usize] = fast_sym as u16;
        }
    }

    0
}

fn huff_build(tree: &mut HuffTree, sym_bits: &[u8], sym_extra: &[u32], sym_extra_offset: u32, fast_bits: u32) -> isize {
    let mut bits_counts = [0u32; HUFF_MAX_BITS];
    for &bits in sym_bits {
        bits_counts[bits as usize] += 1;
    }
    huff_build_imp(tree, sym_bits, sym_extra, sym_extra_offset, fast_bits, &bits_counts)
}

#[inline(always)]
fn huff_decode_bits(tree: &HuffTree, bits: u64, fast_bits: u32, fast_mask: u32) -> u16 {
    let sym = tree.fast_sym[(bits & fast_mask as u64) as usize];
    if (sym & (SYM_FAST | SYM_END)) != 0 {
        return sym;
    }

    let mut tail = (bits >> fast_bits) as u32;
    let long_mask = huff_sym_long_mask(sym);
    if long_mask != 0 {
        return tree.long_sym[(huff_sym_long_offset(sym) + (tail & long_mask)) as usize];
    }

    let mut code = huff_sym_value(sym);
    let mut num_bits = fast_bits as usize;
    loop {
        code = code << 1 | (tail & 1);
        tail >>= 1;
        num_bits += 1;

        // a full code ends before 16 bits
        let Some(&past_max) = tree.past_max_code.get(num_bits) else { return ERROR_SYM };
        if code < past_max as u32 {
            let sorted = (code as i32).wrapping_add(tree.code_to_sorted[num_bits] as i32);
            return tree.sorted_to_sym.get(sorted as usize).copied().unwrap_or(ERROR_SYM);
        }
    }
}

/// `internal_fast_bits` 0 for the default
fn init_static_huff(trees: &mut Trees, internal_fast_bits: usize) {
    trees.fast_bits = if internal_fast_bits != 0 { internal_fast_bits as u32 } else { HUFF_FAST_BITS };

    let mut lit_length_bits = [0u8; 288];
    lit_length_bits[0..144].fill(8);
    lit_length_bits[144..256].fill(9);
    lit_length_bits[256..280].fill(7);
    lit_length_bits[280..288].fill(8);
    huff_build(&mut trees.lit_length, &lit_length_bits, &LENGTH_LUT, 256, trees.fast_bits);

    let dist_bits = [5u8; 32];
    huff_build(&mut trees.dist, &dist_bits, &DIST_LUT, 0, trees.fast_bits);
}

fn decode_dynamic_huff_bits(dc: &mut DeflateContext, huff_code_length: &HuffTree, code_lengths: &mut [u8; HUFF_MAX_COMBINED_SYMS], num_symbols: u32) -> isize {
    let mut bits = dc.stream.bits;
    let mut left = dc.stream.left;
    let mut data = dc.stream.chunk_ptr;

    let mut symbol_index = 0u32;
    let mut prev = 0u8;
    while symbol_index < num_symbols {
        dc.stream.bit_refill(&mut bits, &mut left, &mut data);
        if dc.stream.stop_error != 0 {
            return dc.stream.stop_error as isize;
        }

        let sym = huff_decode_bits(huff_code_length, bits, HUFF_CODELEN_FAST_BITS, HUFF_CODELEN_FAST_MASK);
        if sym == ERROR_SYM {
            return -21;
        }

        let inst = huff_sym_value(sym);
        let sym_len = huff_sym_total_bits(sym);

        bits >>= sym_len;
        left = left.wrapping_sub(sym_len as usize);

        if inst <= 15 {
            prev = inst as u8;
            code_lengths[symbol_index as usize] = inst as u8;
            symbol_index += 1;
        } else if inst == 16 {
            let num = 3 + (bits as u32 & 0x3);
            bits >>= 2;
            left = left.wrapping_sub(2);
            if symbol_index + num > num_symbols {
                return -18;
            }
            code_lengths[symbol_index as usize..(symbol_index + num) as usize].fill(prev);
            symbol_index += num;
        } else if inst == 17 {
            let num = 3 + (bits as u32 & 0x7);
            bits >>= 3;
            left = left.wrapping_sub(3);
            if symbol_index + num > num_symbols {
                return -19;
            }
            code_lengths[symbol_index as usize..(symbol_index + num) as usize].fill(0);
            symbol_index += num;
            prev = 0;
        } else if inst == 18 {
            let num = 11 + (bits as u32 & 0x7f);
            bits >>= 7;
            left = left.wrapping_sub(7);
            if symbol_index + num > num_symbols {
                return -20;
            }
            code_lengths[symbol_index as usize..(symbol_index + num) as usize].fill(0);
            symbol_index += num;
            prev = 0;
        } else {
            return -6;
        }
    }

    dc.stream.bits = bits;
    dc.stream.left = left;
    dc.stream.chunk_ptr = data;
    0
}

fn init_dynamic_huff(dc: &mut DeflateContext, trees: &mut Trees) -> isize {
    let mut bits = dc.stream.bits;
    let mut left = dc.stream.left;
    let mut data = dc.stream.chunk_ptr;
    dc.stream.bit_refill(&mut bits, &mut left, &mut data);
    if dc.stream.stop_error != 0 {
        return dc.stream.stop_error as isize;
    }

    trees.fast_bits = dc.fast_bits;

    let num_lit_lengths = 257 + (bits & 0x1f) as u32;
    let num_dists = 1 + (bits >> 5 & 0x1f) as u32;
    let num_code_lengths = 4 + (bits >> 10 & 0xf) as usize;
    bits >>= 14;
    left = left.wrapping_sub(14);

    let mut code_lengths = [0u8; HUFF_MAX_COMBINED_SYMS];

    for len_i in 0..num_code_lengths {
        if len_i == 14 {
            dc.stream.bit_refill(&mut bits, &mut left, &mut data);
            if dc.stream.stop_error != 0 {
                return dc.stream.stop_error as isize;
            }
        }
        code_lengths[CODE_LENGTH_PERMUTATION[len_i] as usize] = (bits as u32 & 0x7) as u8;
        bits >>= 3;
        left = left.wrapping_sub(3);
    }

    dc.stream.bits = bits;
    dc.stream.left = left;
    dc.stream.chunk_ptr = data;

    let mut huff_code_length = HuffTree::ZERO;

    let err = huff_build(&mut huff_code_length, &code_lengths[..HUFF_CODELEN_SYMS], &[], i32::MAX as u32, HUFF_CODELEN_FAST_BITS);
    if err != 0 {
        return -14 + 1 + err;
    }

    let err = decode_dynamic_huff_bits(dc, &huff_code_length, &mut code_lengths, num_lit_lengths + num_dists);
    if err != 0 {
        return err;
    }

    let num_lit_lengths = num_lit_lengths as usize;
    let err = huff_build(&mut trees.lit_length, &code_lengths[..num_lit_lengths], &LENGTH_LUT, 256, dc.fast_bits);
    if err != 0 {
        return if err == -7 { -28 } else { -16 + 1 + err };
    }

    let err = huff_build(&mut trees.dist, &code_lengths[num_lit_lengths..num_lit_lengths + num_dists as usize], &DIST_LUT, 0, dc.fast_bits);
    if err != 0 {
        return if err == -7 { -28 } else { -22 + 1 + err };
    }

    0
}

fn adler32(data: &[u8]) -> u32 {
    // ufbx sums with SSE2; here sixteen lanes the compiler keeps in vector registers do, in 16 bits for 256 bytes, in
    // 32 bits for blocks of 5552 bytes, the most whose sums cannot wrap
    const MOD: u64 = 65521;
    let (mut a, mut b) = (1u64, 0u64);
    for block in data.chunks(5552) {
        let (chunks, rest) = block.as_chunks::<16>();
        let steps = chunks.len() as u64;
        let (mut column, mut before) = ([0u32; 16], [0u32; 16]);
        for group in chunks.chunks(16) {
            let (mut column16, mut before16) = ([0u16; 16], [0u16; 16]);
            for chunk in group {
                for i in 0..16 {
                    before16[i] += column16[i];
                    column16[i] += chunk[i] as u16;
                }
            }
            let n = group.len() as u32;
            for i in 0..16 {
                before[i] += n * column[i] + before16[i] as u32;
                column[i] += column16[i] as u32;
            }
        }
        let sum: u64 = column.iter().map(|&x| x as u64).sum();
        let weighted: u64 = column.iter().zip((1..=16u64).rev()).map(|(&x, w)| x as u64 * w).sum();
        let before: u64 = before.iter().map(|&x| x as u64).sum();
        b += 16 * steps * a + 16 * before + weighted;
        a += sum;
        for &x in rest {
            a += x as u64;
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16 | a) as u32
}

/// One 16-byte load and store, as ufbx's SSE build does
#[inline(always)]
fn copy_16_bytes(out: &mut [u8], dst: usize, src: usize) {
    out.copy_within(src..src + 16, dst);
}

/// 0 at the end of the block, 1 after `max_symbols`, or an error
fn inflate_block_slow(dc: &mut DeflateContext, out: &mut [u8], trees: &Trees, mut max_symbols: usize) -> isize {
    let mut out_ptr = dc.out_ptr;
    let out_end = out.len();

    let fast_bits = trees.fast_bits;
    let fast_mask = (1u32 << fast_bits) - 1;

    let mut bits = dc.stream.bits;
    let mut left = dc.stream.left;
    let mut data = dc.stream.chunk_ptr;

    loop {
        if max_symbols == 0 {
            break;
        }
        max_symbols -= 1;

        dc.stream.bit_refill(&mut bits, &mut left, &mut data);
        if dc.stream.stop_error != 0 {
            return dc.stream.stop_error as isize;
        }
        let sym_bits = bits;

        let sym0 = huff_decode_bits(&trees.lit_length, bits, fast_bits, fast_mask);
        let sym0_bits = huff_sym_total_bits(sym0);

        bits >>= sym0_bits;
        left = left.wrapping_sub(sym0_bits as usize);
        if (sym0 & SYM_END) != 0 {
            if huff_sym_value(sym0) != 0 {
                return -13;
            }

            dc.out_ptr = out_ptr;
            dc.stream.bits = bits;
            dc.stream.left = left;
            dc.stream.chunk_ptr = data;
            return 0;
        } else if (sym0 & SYM_MATCH) == 0 {
            if out_ptr == out_end {
                return -10;
            }
            out[out_ptr] = huff_sym_value(sym0) as u8;
            out_ptr += 1;
            continue;
        }

        let sym0_value = huff_sym_value(sym0) as usize;
        let len_shift_base = trees.lit_length.extra_shift_base[sym0_value];
        let len_mask = trees.lit_length.extra_mask[sym0_value];
        let mut length = (len_shift_base >> 16).wrapping_add((sym_bits.wrapping_shr(len_shift_base) & len_mask as u64) as u32);

        let sym1 = huff_decode_bits(&trees.dist, bits, fast_bits, fast_mask);
        if (sym1 & SYM_END) != 0 {
            return -11;
        }

        let sym1_bits = huff_sym_total_bits(sym1);

        bits >>= sym1_bits;
        left = left.wrapping_sub(sym1_bits as usize);

        let sym1_value = huff_sym_value(sym1) as usize;
        let dist_shift_base = trees.dist.extra_shift_base[sym1_value];
        let dist_mask = trees.dist.extra_mask[sym1_value];
        let distance = (dist_shift_base >> 16).wrapping_add((sym_bits.wrapping_shr(dist_shift_base.wrapping_add(sym0 as u32)) & dist_mask as u64) as u32);

        let out_space = out_end - out_ptr;
        if distance as usize > out_ptr || length as usize > out_space {
            return -12;
        }

        let mut src = out_ptr - distance as usize;
        let mut dst = out_ptr;
        let end = dst + length as usize;
        out_ptr += length as usize;

        if out_space >= length as usize + 16 {
            let min_dist = if length < 16 { length } else { 16 };
            if distance >= min_dist {
                copy_16_bytes(out, dst, src);
                while length > 16 {
                    src += 16;
                    dst += 16;
                    length -= 16;
                    copy_16_bytes(out, dst, src);
                }
            } else {
                while dst != end {
                    out[dst] = out[src];
                    dst += 1;
                    src += 1;
                }
            }
        } else {
            while dst != end {
                out[dst] = out[src];
                dst += 1;
                src += 1;
            }
        }
    }

    dc.out_ptr = out_ptr;
    dc.stream.bits = bits;
    dc.stream.left = left;
    dc.stream.chunk_ptr = data;
    1
}

/// For 10 fast bits, while 8 bytes of input are left before the yield and 2 of output
fn inflate_block_fast(dc: &mut DeflateContext, out: &mut [u8], trees: &Trees) -> isize {
    let mut out_ptr = dc.out_ptr;
    let out_end = out.len() as isize - FAST_MIN_OUT;

    let tree_lit_length = &trees.lit_length;
    let tree_dist = &trees.dist;

    let mut bits = dc.stream.bits;
    let mut left = dc.stream.left;
    let mut data = dc.stream.chunk_ptr;
    let data_end = dc.stream.chunk_yield.wrapping_sub(FAST_MIN_IN);
    let chunk = dc.stream.chunk_bytes();

    let mut sym01_bits: u64;
    let mut sym0: u16;
    let mut sym1: u16;
    let mut refill_bits = read_u64(chunk, data);

    macro_rules! refill_and_decode {
        () => {
            bits |= refill_bits.wrapping_shl(left as u32);
            data += 63usize.wrapping_sub(left) >> 3;
            left |= 56;
            sym01_bits = bits;
            sym0 = tree_lit_length.fast_sym[(sym01_bits & HUFF_FAST_MASK) as usize];
            sym1 =
                (if (sym0 & SYM_MATCH) != 0 { tree_dist } else { tree_lit_length }).fast_sym[(sym01_bits.wrapping_shr(sym0 as u32) & HUFF_FAST_MASK) as usize];
            refill_bits = read_u64(chunk, data);
        };
    }
    macro_rules! should_continue {
        () => {
            ((data_end.wrapping_sub(data) as isize) | (out_end - out_ptr as isize)) >= 0
        };
    }

    refill_and_decode!();

    loop {
        if (sym0 & sym1) & SYM_FAST != 0 {
            bits = sym01_bits.wrapping_shr(sym0 as u32 + sym1 as u32);
            left = left.wrapping_sub(((sym0 as u32 + sym1 as u32) & 0x3f) as usize);

            if ((sym0 | sym1) & SYM_MATCH) == 0 {
                out[out_ptr] = huff_sym_value(sym0) as u8;
                out[out_ptr + 1] = huff_sym_value(sym1) as u8;
                out_ptr += 2;

                refill_and_decode!();
                if should_continue!() {
                    continue;
                }
                break;
            } else if (sym0 & SYM_MATCH) == 0 {
                out[out_ptr] = huff_sym_value(sym0) as u8;
                out_ptr += 1;

                sym01_bits = sym01_bits.wrapping_shr(sym0 as u32);

                sym0 = sym1;
                sym1 = tree_dist.fast_sym[(bits & HUFF_FAST_MASK) as usize];

                if (sym1 & SYM_FAST) == 0 {
                    if (sym1 & SYM_END) != 0 {
                        return -11;
                    }
                    let tail = (bits >> HUFF_FAST_BITS) as u32;
                    let long_mask = huff_sym_long_mask(sym1);
                    sym1 = tree_dist.long_sym[(huff_sym_long_offset(sym1) + (tail & long_mask)) as usize];
                    if (sym1 & SYM_END) != 0 {
                        return -11;
                    }
                }

                bits = bits.wrapping_shr(sym1 as u32);
                left = left.wrapping_sub((sym1 & 0x3f) as usize);
            }
        } else {
            if (sym0 & (SYM_FAST | SYM_END)) == 0 {
                let tail = (sym01_bits >> HUFF_FAST_BITS) as u32;
                let long_mask = huff_sym_long_mask(sym0);
                sym0 = tree_lit_length.long_sym[(huff_sym_long_offset(sym0) + (tail & long_mask)) as usize];
            }

            let sym0_bits = huff_sym_total_bits(sym0);
            bits >>= sym0_bits;
            left = left.wrapping_sub(sym0_bits as usize);

            if (sym0 & SYM_END) != 0 {
                if huff_sym_value(sym0) != 0 {
                    return -13;
                }
                dc.out_ptr = out_ptr;
                dc.stream.bits = bits;
                dc.stream.left = left;
                dc.stream.chunk_ptr = data;
                return 0;
            }

            if (sym0 & SYM_MATCH) != 0 {
                sym1 = tree_dist.fast_sym[(bits & HUFF_FAST_MASK) as usize];

                if (sym1 & SYM_FAST) == 0 {
                    if (sym1 & SYM_END) != 0 {
                        return -11;
                    }
                    let tail = (bits >> HUFF_FAST_BITS) as u32;
                    let long_mask = huff_sym_long_mask(sym1);
                    sym1 = tree_dist.long_sym[(huff_sym_long_offset(sym1) + (tail & long_mask)) as usize];
                    if (sym1 & SYM_END) != 0 {
                        return -11;
                    }
                }

                bits = bits.wrapping_shr(sym1 as u32);
                left = left.wrapping_sub((sym1 & 0x3f) as usize);
            } else {
                out[out_ptr] = huff_sym_value(sym0) as u8;
                out_ptr += 1;

                refill_and_decode!();
                if should_continue!() {
                    continue;
                }
                break;
            }
        }

        // a match's value is its slot in the tables, below 32 as `huff_build_imp` numbers them: the mask spares a bounds check
        let sym0_value = huff_sym_value(sym0) as usize & (HUFF_MAX_EXTRA_SYMS - 1);
        let len_shift_base = trees.lit_length.extra_shift_base[sym0_value];
        let len_mask = trees.lit_length.extra_mask[sym0_value];
        let mut length = (len_shift_base >> 16).wrapping_add((sym01_bits.wrapping_shr(len_shift_base) & len_mask as u64) as u32);

        let sym1_value = huff_sym_value(sym1) as usize & (HUFF_MAX_EXTRA_SYMS - 1);
        let dist_shift_base = trees.dist.extra_shift_base[sym1_value];
        let dist_mask = trees.dist.extra_mask[sym1_value];
        let distance = (dist_shift_base >> 16).wrapping_add((sym01_bits.wrapping_shr(dist_shift_base.wrapping_add(sym0 as u32)) & dist_mask as u64) as u32);

        refill_and_decode!();

        let dst_space = out_end - out_ptr as isize - length as isize + FAST_MIN_OUT;
        let src_space = out_ptr as isize - distance as isize;
        if (dst_space | src_space) < 0 {
            return -12;
        }

        let mut src = out_ptr - distance as usize;
        let mut dst = out_ptr;
        let end = dst + length as usize;
        out_ptr += length as usize;

        let min_dist = if length < 16 { length } else { 16 };
        if distance >= min_dist && dst_space >= 16 {
            copy_16_bytes(out, dst, src);
            while length > 16 {
                src += 16;
                dst += 16;
                length -= 16;
                copy_16_bytes(out, dst, src);
            }
        } else {
            while dst != end {
                out[dst] = out[src];
                dst += 1;
                src += 1;
            }
        }

        if should_continue!() {
            continue;
        }
        break;
    }

    dc.out_ptr = out_ptr;
    dc.stream.bits = bits;
    dc.stream.left = left;
    dc.stream.chunk_ptr = data;
    1
}

/// The bytes written to `dst`, or ufbx's negative error code (-28 cancelled, -31 truncated, ...)
pub fn inflate(dst: &mut [u8], input: &mut InflateInput, retain: &mut InflateRetain) -> isize {
    let total_size = input.total_size;
    let internal_fast_bits = input.internal_fast_bits;
    let no_header = input.no_header;
    let no_checksum = input.no_checksum;

    let mut dc = DeflateContext { stream: BitStream::bit_stream_init(input), fast_bits: 0, out_ptr: 0 };
    let out_end = dst.len();
    if internal_fast_bits != 0 {
        dc.fast_bits = internal_fast_bits as u32;
        if dc.fast_bits < 1 || dc.fast_bits == 9 || dc.fast_bits > 10 {
            return -29;
        }
    } else {
        dc.fast_bits = if total_size > 2048 { 10 } else { 8 };
    }

    let mut bits = dc.stream.bits;
    let mut left = dc.stream.left;
    let mut data = dc.stream.chunk_ptr;

    dc.stream.bit_refill(&mut bits, &mut left, &mut data);
    if dc.stream.stop_error != 0 {
        return dc.stream.stop_error as isize;
    }

    if !no_header {
        let cmf = (bits & 0xff) as usize;
        let flg = ((bits >> 8) & 0xff) as usize;
        bits >>= 16;
        left = left.wrapping_sub(16);

        if (cmf & 0xf) != 0x8 {
            return -1;
        }
        if (flg & 0x20) != 0 {
            return -2;
        }
        if !(cmf << 8 | flg).is_multiple_of(31) {
            return -3;
        }
        if (cmf >> 4) > 7 {
            return -30;
        }
    }

    let mut tree_data: Option<Box<Trees>> = None;
    loop {
        dc.stream.bit_refill(&mut bits, &mut left, &mut data);
        if dc.stream.stop_error != 0 {
            return dc.stream.stop_error as isize;
        }

        let header = (bits & 0x7) as usize;
        bits >>= 3;
        left = left.wrapping_sub(3);

        let block_type = header >> 1;
        if block_type == 0 {
            let align_bits = left & 0x7;
            bits >>= align_bits;
            left -= align_bits;

            let len = (bits & 0xffff) as usize;
            let nlen = ((bits >> 16) & 0xffff) as usize;
            if (len ^ nlen) != 0xffff {
                return -4;
            }
            if out_end - dc.out_ptr < len {
                return -6;
            }
            bits >>= 32;
            left = left.wrapping_sub(32);

            dc.stream.bits = bits;
            dc.stream.left = left;
            dc.stream.chunk_ptr = data;

            if !dc.stream.bit_copy_bytes(&mut dst[dc.out_ptr..dc.out_ptr + len]) {
                return -5;
            }

            dc.out_ptr += len;
        } else if block_type <= 2 {
            dc.stream.bits = bits;
            dc.stream.left = left;
            dc.stream.chunk_ptr = data;

            let trees: &Trees = if block_type == 1 {
                // built once and kept in `retain`, with the fast bits of the call that built them
                retain.static_trees.get_or_insert_with(|| {
                    let mut trees = Box::new(Trees::ZERO);
                    init_static_huff(&mut trees, internal_fast_bits);
                    trees
                })
            } else {
                let trees = tree_data.get_or_insert_with(|| Box::new(Trees::ZERO));
                let err = init_dynamic_huff(&mut dc, trees);
                if err != 0 {
                    return err;
                }
                trees
            };

            loop {
                let fast_viable = trees.fast_bits == HUFF_FAST_BITS && (out_end - dc.out_ptr) as isize >= FAST_MIN_OUT;

                let err = if fast_viable && (dc.stream.chunk_yield.wrapping_sub(dc.stream.chunk_ptr) as isize) >= FAST_MIN_IN as isize {
                    inflate_block_fast(&mut dc, dst, trees)
                } else {
                    inflate_block_slow(&mut dc, dst, trees, if fast_viable { 32 } else { usize::MAX })
                };

                if err < 0 {
                    return err;
                }

                // a cancel returns normally from the block
                if dc.stream.stop_error != 0 {
                    return dc.stream.stop_error as isize;
                }

                if err == 0 {
                    break;
                }
            }
        } else {
            return -7;
        }

        bits = dc.stream.bits;
        left = dc.stream.left;
        data = dc.stream.chunk_ptr;

        if (header & 1) != 0 {
            break;
        }
    }

    {
        let align_bits = left & 0x7;
        bits >>= align_bits;
        left -= align_bits;
        dc.stream.bit_refill(&mut bits, &mut left, &mut data);
        if dc.stream.stop_error != 0 {
            return dc.stream.stop_error as isize;
        }

        if !no_checksum {
            let reference = (bits as u32).swap_bytes();
            let checksum = adler32(&dst[..dc.out_ptr]);
            if reference != checksum {
                return -9;
            }
        }
    }

    dc.out_ptr as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    // zlib.compress(b"hello hello hello hello", 9): one fixed-Huffman block
    const FIXED: [u8; 16] = [0x78, 0xda, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08, 0xb1];
    // zlib.compress(dynamic_text(), 9): one dynamic-Huffman block
    const DYNAMIC: &str =
        "78dae5cc810dc0200800b05bd944200a6250787f87ac0714b847fa031656b0ce6e781be862b7974024df642e84a938e594570e93ed34ba3ba9c4d5d848f08fe40357c27e7e";

    fn dynamic_text() -> Vec<u8> {
        (0..300u32).map(|i| ((i * i * 7 + i / 3) % 23 + 97) as u8).collect()
    }

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn adler32_reference(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &x in data {
            a = (a + x as u32) % 65521;
            b = (b + a) % 65521;
        }
        b << 16 | a
    }

    fn noise(n: usize, seed: u64) -> Vec<u8> {
        let mut x = seed;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (x >> 33) as u8
            })
            .collect()
    }

    fn stored_zlib(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x78, 0x01];
        let chunks: Vec<&[u8]> = data.chunks(65535).collect();
        for (i, c) in chunks.iter().enumerate() {
            out.push((i + 1 == chunks.len()) as u8);
            out.extend_from_slice(&(c.len() as u16).to_le_bytes());
            out.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
            out.extend_from_slice(c);
        }
        out.extend_from_slice(&adler32_reference(data).to_be_bytes());
        out
    }

    fn run(z: &[u8], dst: &mut [u8]) -> isize {
        inflate(dst, &mut InflateInput { total_size: z.len(), data: z, ..Default::default() }, &mut InflateRetain::new())
    }

    #[test]
    fn adler32_matches_the_definition() {
        let buf = noise(20000, 7);
        for &(start, len) in &[(0, 0), (0, 1), (3, 15), (1, 16), (5, 511), (7, 512), (0, 4099), (9, 19990)] {
            assert_eq!(adler32(&buf[start..start + len]), adler32_reference(&buf[start..start + len]));
        }
        let ones = vec![0xffu8; 100000];
        assert_eq!(adler32(&ones), adler32_reference(&ones));
    }

    #[test]
    fn stored_blocks() {
        let data = noise(70000, 1);
        let mut out = vec![0u8; data.len()];
        assert_eq!(run(&stored_zlib(&data), &mut out), data.len() as isize);
        assert_eq!(out, data);
    }

    #[test]
    fn fixed_and_dynamic_blocks() {
        let mut out = [0u8; 64];
        assert_eq!(run(&FIXED, &mut out), 23);
        assert_eq!(&out[..23], b"hello hello hello hello");
        let text = dynamic_text();
        let mut out = vec![0u8; 300];
        assert_eq!(run(&unhex(DYNAMIC), &mut out), 300);
        assert_eq!(out, text);
    }

    #[test]
    fn data_then_reader() {
        let z = unhex(DYNAMIC);
        let mut pos = 5;
        let mut reader = |buf: &mut [u8]| {
            let n = buf.len().min(z.len() - pos);
            buf[..n].copy_from_slice(&z[pos..pos + n]);
            pos += n;
            n
        };
        let mut out = vec![0u8; 300];
        let mut input = InflateInput { total_size: z.len(), data: &z[..5], read_fn: Some(&mut reader), ..Default::default() };
        assert_eq!(inflate(&mut out, &mut input, &mut InflateRetain::new()), 300);
        assert_eq!(out, dynamic_text());
    }

    #[test]
    fn error_codes() {
        let mut out = [0u8; 64];
        let mut bad = FIXED;
        bad[0] = 0x79;
        assert_eq!(run(&bad, &mut out), -1);
        let mut bad = FIXED;
        bad[15] ^= 1;
        assert_eq!(run(&bad, &mut out), -9);
        let stored = stored_zlib(&noise(1000, 3));
        assert_eq!(run(&stored[..500], &mut vec![0u8; 1000]), -5);
        assert_eq!(run(&FIXED, &mut out[..3]), -10);
        assert_eq!(run(&FIXED, &mut out[..10]), -12);
        let input = &mut InflateInput { total_size: FIXED.len(), data: &FIXED, internal_fast_bits: 9, ..Default::default() };
        assert_eq!(inflate(&mut out, input, &mut InflateRetain::new()), -29);
        let cancel = |_: &Progress| ProgressResult::Cancel;
        let input = &mut InflateInput { total_size: FIXED.len(), data: &FIXED, progress_cb: Some(&cancel), progress_interval_hint: 1, ..Default::default() };
        assert_eq!(inflate(&mut out, input, &mut InflateRetain::new()), -28);
    }
}
