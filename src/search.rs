//! Ported as they run, so that lists ufbx searches without their being sorted (a shader texture's inputs) and comparisons
//! that are no strict order (NaN weights) give ufbx's answers.

/// The first index in `[lo, hi)` where `eq` holds, after the binary search by `less`
pub(crate) fn lower_bound_eq(mut lo: usize, mut hi: usize, linear: usize, less: impl Fn(usize) -> bool, eq: impl Fn(usize) -> bool) -> Option<usize> {
    while hi - lo > linear {
        let mid = lo + (hi - lo) / 2;
        if less(mid) {
            lo = mid + 1;
        } else {
            hi = mid + 1;
        }
    }
    (lo..hi).find(|&i| eq(i))
}

pub(crate) fn upper_bound_eq(mut lo: usize, mut hi: usize, linear: usize, eq: impl Fn(usize) -> bool) -> usize {
    let mut step = 1;
    while step < 100 && hi - lo > step {
        if !eq(lo + step) {
            hi = lo + step;
            break;
        }
        lo += step;
        step *= 2;
    }
    while hi - lo > linear {
        let mid = lo + (hi - lo) / 2;
        if eq(mid) {
            lo = mid + 1;
        } else {
            hi = mid + 1;
        }
    }
    while lo < hi {
        if !eq(lo) {
            break;
        }
        lo += 1;
    }
    lo
}

/// `ufbxi_macro_stable_sort` in ufbx.c: The order is found on indices, so that the entries are moved once and never copied
pub(crate) fn stable_sort<T>(data: &mut [T], linear: usize, less: impl Fn(&T, &T) -> bool) {
    let size = data.len();
    if size < 2 {
        return;
    }
    if size <= linear {
        for i in 1..size {
            let mut j = i;
            while j != 0 && less(&data[i], &data[j - 1]) {
                j -= 1;
            }
            data[j..=i].rotate_right(1);
        }
        return;
    }
    let mut to: Vec<usize> = (0..size).collect();
    let mut block = linear;
    let mut base = 0;
    while base < size {
        let end = (base + block).min(size);
        for i in base + 1..end {
            let x = to[i];
            let mut j = i;
            while j != base {
                if !less(&data[x], &data[to[j - 1]]) {
                    break;
                }
                to[j] = to[j - 1];
                j -= 1;
            }
            to[j] = x;
        }
        base += block;
    }
    let mut from = vec![0; size];
    while block < size {
        std::mem::swap(&mut from, &mut to);
        let mut base = 0;
        while base < size {
            let mut i = base;
            let i_end = (base + block).min(size);
            let mut j = base + block;
            let j_end = (j + block).min(size);
            let mut k = base;
            while i < i_end && j < j_end {
                if less(&data[from[j]], &data[from[i]]) {
                    to[k] = from[j];
                    j += 1;
                } else {
                    to[k] = from[i];
                    i += 1;
                }
                k += 1;
            }
            while i < i_end {
                to[k] = from[i];
                k += 1;
                i += 1;
            }
            while j < j_end {
                to[k] = from[j];
                k += 1;
                j += 1;
            }
            base += block * 2;
        }
        block *= 2;
    }
    // entry `k` is to come from `to[k]`: each cycle of that permutation is closed by swaps
    for start in 0..size {
        let mut k = start;
        while to[k] != start {
            let next = to[k];
            data.swap(k, next);
            to[k] = k;
            k = next;
        }
        to[k] = k;
    }
}

/// A heap sort: where it leaves entries that compare equal is part of ufbx's result
pub(crate) fn unstable_sort<T>(data: &mut [T], less: impl Fn(&T, &T) -> bool) {
    let size = data.len();
    if size <= 1 {
        return;
    }

    let mut start = (size - 1) >> 1;
    let mut end = size - 1;
    loop {
        let mut root = start;
        loop {
            let child = root * 2 + 1;
            if child > end {
                break;
            }
            let mut next = if less(&data[child], &data[root]) { root } else { child };
            if child < end && less(&data[next], &data[child + 1]) {
                next = child + 1;
            }
            if next == root {
                break;
            }
            data.swap(root, next);
            root = next;
        }

        if start > 0 {
            start -= 1;
        } else if end > 0 {
            data.swap(end, 0);
            end -= 1;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sort_is_stable_and_the_bounds_find_the_run() {
        let mut v: Vec<(u32, u32)> = (0..200).map(|i| ((i * 7919) % 13, i)).collect();
        let mut w = v.clone();
        stable_sort(&mut v, 32, |a, b| a.0 < b.0);
        w.sort_by_key(|x| x.0);
        assert_eq!(v, w);
        let keys: Vec<u32> = v.iter().map(|x| x.0).collect();
        let lo = lower_bound_eq(0, keys.len(), 32, |i| keys[i] < 5, |i| keys[i] == 5).unwrap();
        let hi = upper_bound_eq(lo, keys.len(), 32, |i| keys[i] == 5);
        assert_eq!((lo, hi), (keys.partition_point(|&k| k < 5), keys.partition_point(|&k| k <= 5)));
    }

    fn xorshift32(state: &mut u32) -> u32 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *state = x;
        x
    }

    fn generate_linear(size: usize, start: u32, delta: u32) -> Vec<u32> {
        (0..size as u32).map(|i| start.wrapping_add(delta.wrapping_mul(i))).collect()
    }

    fn generate_random(size: usize, seed: u32, modulus: u32) -> Vec<u32> {
        let mut state = seed | 1;
        (0..size).map(|_| xorshift32(&mut state) % modulus).collect()
    }

    fn sort_by<T: Clone>(stable: bool, linear: usize, data: &mut [T], less: impl Fn(&T, &T) -> bool) {
        if stable { stable_sort(data, linear, less) } else { unstable_sort(data, less) }
    }

    fn find_uint(data: &[u32], linear: usize, value: u32) -> Option<usize> {
        lower_bound_eq(0, data.len(), linear, |i| data[i] < value, |i| data[i] == value)
    }

    fn check_sort(mut data: Vec<u32>, stable: bool, linear: usize) {
        let size = data.len();
        let mut pairs: Vec<(u32, u32)> = data.iter().enumerate().map(|(i, &a)| (a, i as u32)).collect();
        sort_by(stable, linear, &mut data, |a, b| a < b);
        sort_by(stable, linear, &mut pairs, |a, b| a.0 < b.0);
        for i in 1..size {
            assert!(data[i - 1] <= data[i]);
            assert!(pairs[i - 1].0 <= pairs[i].0);
            if pairs[i - 1].0 == pairs[i].0 && stable {
                assert!(pairs[i - 1].1 < pairs[i].1);
            }
        }
        for i in 0..size {
            let value = data[i];
            let index = find_uint(&data, linear, value).unwrap();
            assert!(index <= i && data[index] == value);
            assert_eq!(Some(index), lower_bound_eq(0, size, linear, |k| pairs[k].0 < value, |k| pairs[k].0 == value));
            if index > 0 {
                assert!(data[index - 1] < value);
            }
            let end = upper_bound_eq(index, size, linear, |k| data[k] == value);
            assert!(end > i && data[end - 1] == value);
            if end < size {
                assert!(data[end] > value);
            }
        }
        assert_eq!(find_uint(&data, linear, u32::MAX), None);
        sort_by(stable, linear, &mut pairs, |a, b| a.1 < b.1);
        assert!(pairs.iter().enumerate().all(|(i, p)| p.1 == i as u32));
    }

    fn check_sort_strings(data: &[u32], stable: bool, linear: usize) {
        let mut strings: Vec<String> = data.iter().map(|v| v.to_string()).collect();
        sort_by(stable, linear, &mut strings, |a, b| a < b);
        assert!(strings.windows(2).all(|w| w[0] <= w[1]));
        for v in data {
            let find = v.to_string();
            let index = lower_bound_eq(0, strings.len(), linear, |k| strings[k] < find, |k| strings[k] == find).unwrap();
            if index > 0 {
                assert!(strings[index - 1] < find);
            }
        }
    }

    #[test]
    fn test_sorts() {
        const MAX_SORT_SIZE: usize = 2048;
        for stable in [true, false] {
            let mut linear = 2;
            while linear <= 64 {
                let mut size = 0;
                while size < MAX_SORT_SIZE {
                    check_sort(generate_linear(size, 0, 1), stable, linear);
                    check_sort(generate_linear(size, size as u32, u32::MAX), stable, linear);
                    check_sort(generate_random(size, size as u32, 1 + size as u32 % 10), stable, linear);
                    check_sort(generate_random(size, size as u32, u32::MAX), stable, linear);
                    size += 1 + size / 128 + size / 512 * 32;
                }
                let size = MAX_SORT_SIZE;
                check_sort_strings(&generate_linear(size, 0, 1), stable, linear);
                check_sort_strings(&generate_linear(size, size as u32, u32::MAX), stable, linear);
                check_sort_strings(&generate_random(size, size as u32, 1 + size as u32 % 10), stable, linear);
                check_sort_strings(&generate_random(size, size as u32, u32::MAX), stable, linear);
                linear += 1 + linear / 8;
            }
        }
    }
}
