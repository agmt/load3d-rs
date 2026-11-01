//! `cargo run --example flatten --features flatten -- <file>`: each mesh's triangles where the file places them, with
//! their materials' colours, whatever the file's format.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: flatten <file>")?;
    let scene = load3d::load_file(&path, &load3d::LoadOpts::default())?;
    for part in load3d::flatten(&scene) {
        println!("{}: {} triangles", scene.nodes[part.node].element.name, part.triangles.len());
        for (triangle, material) in part.triangles.iter().zip(&part.materials).take(3) {
            let corners = triangle.map(|v| part.positions[v as usize]);
            let colour = material.and_then(|m| scene.materials[m].diffuse());
            println!("  {corners:?}, colour {colour:?}");
        }
    }
    Ok(())
}
