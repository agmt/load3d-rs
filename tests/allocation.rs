//! A count a file states is not allocated before its values are read: the browser's 32-bit build cannot allocate four
//! billion of them, where a 64-bit one reserves the room without touching it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

const MOST: usize = 1 << 30;

struct Capped;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        if layout.size() > MOST { std::ptr::null_mut() } else { unsafe { System.alloc(layout) } }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        if layout.size() > MOST { std::ptr::null_mut() } else { unsafe { System.alloc_zeroed(layout) } }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        LARGEST.fetch_max(size, Ordering::Relaxed);
        if size > MOST { std::ptr::null_mut() } else { unsafe { System.realloc(ptr, layout, size) } }
    }
}

#[global_allocator]
static ALLOCATOR: Capped = Capped;

fn node(out: &mut Vec<u8>, name: &str, num_values: u32, values: &[u8], children: impl FnOnce(&mut Vec<u8>)) {
    let start = out.len();
    out.extend([0; 12]);
    out.push(name.len() as u8);
    out.extend(name.as_bytes());
    out.extend(values);
    let before = out.len();
    children(out);
    if out.len() > before {
        out.extend([0; 13]);
    }
    let end = out.len() as u32;
    out[start..start + 4].copy_from_slice(&end.to_le_bytes());
    out[start + 4..start + 8].copy_from_slice(&num_values.to_le_bytes());
    out[start + 8..start + 12].copy_from_slice(&(values.len() as u32).to_le_bytes());
}

fn string(s: &str) -> Vec<u8> {
    [&[b'S'][..], &(s.len() as u32).to_le_bytes(), s.as_bytes()].concat()
}

#[test]
fn an_array_of_four_billion_values_in_a_small_file_is_refused_unallocated() {
    let mut file = b"Kaydara FBX Binary  \x00\x1a\x00".to_vec();
    file.extend(7400u32.to_le_bytes());
    node(&mut file, "Objects", 0, &[], |out| {
        let values = [&[b'L'][..], &1u64.to_le_bytes(), &string("a\x00\x01Geometry"), &string("Mesh")].concat();
        node(out, "Geometry", 3, &values, |out| {
            // one double where the node says it holds four billion
            node(out, "Vertices", u32::MAX - 15, &[&[b'D'][..], &1.0f64.to_le_bytes()].concat(), |_| {});
        });
    });
    assert!(load3d::load_memory(&file, &load3d::LoadOpts::default()).is_err());
    assert!(LARGEST.load(Ordering::Relaxed) <= MOST);
}
