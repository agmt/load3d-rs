use std::cell::RefCell;

use load3d::{CoordinateAxes, LoadOpts, Scene, flatten, load_memory};

const GREY: [f32; 3] = [0.7, 0.7, 0.7];

#[derive(Default)]
pub struct Drawing {
    /// Three corners of x, y, z for each triangle
    pub positions: Vec<f32>,
    /// One red, green, blue for each triangle
    pub colors: Vec<f32>,
    pub message: String,
    /// The positions are in metres; else in the file's own units (an .obj or a 3DS states none)
    pub metres: bool,
}

pub fn draw(bytes: &[u8]) -> Drawing {
    let opts = LoadOpts { target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, target_unit_meters: 1.0, ..LoadOpts::default() };
    match load_memory(bytes, &opts) {
        Ok(scene) => triangles(&scene),
        Err(e) => Drawing { message: e.to_string(), ..Drawing::default() },
    }
}

fn triangles(scene: &Scene) -> Drawing {
    let mut out = Drawing { metres: scene.settings.unit_meters > 0.0, ..Drawing::default() };
    for part in flatten(scene) {
        for (triangle, material) in part.triangles.iter().zip(&part.materials) {
            let color = material.and_then(|m| scene.materials[m].diffuse()).map_or(GREY, |d| [d.x as f32, d.y as f32, d.z as f32]);
            for &corner in triangle {
                let p = part.positions[corner as usize];
                out.positions.extend([p.x as f32, p.y as f32, p.z as f32]);
            }
            out.colors.extend(color);
        }
    }
    let warnings: Vec<String> = scene.metadata.warnings.iter().map(|w| w.description.clone()).collect();
    out.message = format!("{:?}{}", scene.metadata.file_format, if warnings.is_empty() { String::new() } else { format!(" · {}", warnings.join(", ")) });
    out
}

// The page's side: it writes the file into memory from `alloc`, calls `load`, and reads the lists `load` left
thread_local! {
    static DRAWN: RefCell<Drawing> = RefCell::default();
    static INPUT: RefCell<Vec<u8>> = RefCell::default();
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|input| {
        *input = vec![0; len];
        input.as_mut_ptr()
    })
}

/// The number of triangles drawn, 0 where the file failed to load
#[unsafe(no_mangle)]
pub extern "C" fn load() -> usize {
    let drawing = INPUT.with_borrow_mut(|input| draw(&std::mem::take(input)));
    DRAWN.with_borrow_mut(|drawn| {
        *drawn = drawing;
        drawn.colors.len() / 3
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn positions() -> *const f32 {
    DRAWN.with_borrow(|d| d.positions.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn colors() -> *const f32 {
    DRAWN.with_borrow(|d| d.colors.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn message() -> *const u8 {
    DRAWN.with_borrow(|d| d.message.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn message_len() -> usize {
    DRAWN.with_borrow(|d| d.message.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn metres() -> bool {
    DRAWN.with_borrow(|d| d.metres)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE_OBJ: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\n\
        f 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n";

    #[test]
    fn a_cube_of_six_quads_is_drawn_as_twelve_grey_triangles() {
        let d = draw(CUBE_OBJ.as_bytes());
        assert_eq!((d.colors.len() / 3, d.positions.len()), (12, 12 * 9));
        assert!(d.colors.chunks(3).all(|c| c == GREY));
        assert!(d.message.starts_with("Obj") && !d.metres, "{}", d.message);
    }

    #[test]
    fn a_file_it_cannot_read_draws_nothing_and_says_why() {
        let d = draw(b"not a 3D file");
        assert!(d.positions.is_empty() && !d.message.is_empty());
    }
}
