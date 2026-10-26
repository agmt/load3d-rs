//! `cargo run --example load -- <file>`: each node that shows a mesh, its vertices placed in the world, z up, metres.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: load <file>")?;
    let scene =
        load3d::load_file(&path, &load3d::LoadOpts { target_axes: load3d::CoordinateAxes::RIGHT_HANDED_Z_UP, target_unit_meters: 1.0, ..Default::default() })?;
    for node in &scene.nodes {
        let Some(mesh) = node.mesh else { continue };
        let mesh = &scene.meshes[mesh];
        let placed: Vec<load3d::Vec3> = mesh.vertices.iter().map(|&v| load3d::transform_position(&node.geometry_to_world, v)).collect();
        println!("{}: {} vertices, {} faces, the first at {:?}", node.element.name, placed.len(), mesh.faces.len(), placed.first());
    }
    Ok(())
}
