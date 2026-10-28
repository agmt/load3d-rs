//! A file is a tree of chunks, each a u16 id and a u32 length that counts its own six bytes; a parent's length covers its
//! children, and a child is cut at its parent's end. The editor's objects are read, not the keyframer's hierarchy (0xB000):
//! a 3DS stores every vertex already placed.

use crate::context::{Context, ElementInfo};
use crate::error::{Error, Res};
use crate::load::setup_root_node;
use crate::mesh::finalize_mesh;
use crate::parse::Parser;
use crate::paths::init_file_paths;
use crate::props::{name_key, sort_properties};
use crate::types::*;

const MAIN: u16 = 0x4D4D;
const EDITOR: u16 = 0x3D3D;
const OBJECT: u16 = 0x4000;
const TRIMESH: u16 = 0x4100;
const VERTICES: u16 = 0x4110;
const FACES: u16 = 0x4120;
const FACE_MATERIAL: u16 = 0x4130;
const TEX_VERTS: u16 = 0x4140;
const SMOOTHING: u16 = 0x4150;
const MESH_MATRIX: u16 = 0x4160;
const MATERIAL: u16 = 0xAFFF;
const MATERIAL_NAME: u16 = 0xA000;
const AMBIENT: u16 = 0xA010;
const DIFFUSE: u16 = 0xA020;
const SPECULAR: u16 = 0xA030;
const TRANSPARENCY: u16 = 0xA050;
const MAP_NAME: u16 = 0xA300;
const COLOUR_F32: u16 = 0x0010;
const COLOUR_U8: u16 = 0x0011;
const COLOUR_U8_GAMMA: u16 = 0x0012;
const COLOUR_F32_GAMMA: u16 = 0x0013;
const PERCENT_U16: u16 = 0x0030;

/// what a face in no material group wears beside faces in one: Assimp's name for its default material
const DEFAULT_MATERIAL: &str = "DefaultMaterial";

const MAPS: [(u16, &str); 5] =
    [(0xA200, "DiffuseColor"), (0xA204, "SpecularColor"), (0xA210, "TransparentColor"), (0xA220, "ReflectionColor"), (0xA230, "Bump")];

const CHUNK_HEADER: usize = 6;

#[derive(Default)]
struct Object {
    name: Vec<u8>,
    positions: Vec<[f32; 3]>,
    faces: Vec<[u16; 3]>,
    uvs: Vec<[f32; 2]>,
    smoothing: Vec<u32>,
    material_groups: Vec<(Vec<u8>, Vec<u16>)>,
    matrix: Option<[f32; 12]>,
}

/// The first plain value of a colour and the first gamma-corrected one, which some exporters write beside or instead
#[derive(Default, Clone, Copy)]
struct Colour {
    plain: Option<[f32; 3]>,
    gamma: Option<[f32; 3]>,
}

impl Colour {
    fn value(&self) -> Option<[f32; 3]> {
        self.plain.or(self.gamma)
    }
}

#[derive(Default, Clone)]
struct Material {
    name: Vec<u8>,
    ambient: Colour,
    diffuse: Colour,
    specular: Colour,
    transparency: Option<u16>,
    maps: Vec<(&'static str, Vec<u8>)>,
}

#[derive(Default)]
struct File {
    objects: Vec<Object>,
    materials: Vec<Material>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(b, at))
}

fn bad(description: &str, id: u16) -> Error {
    Error::new(description).set_err_info(format!("{id:#06x}"))
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    taken: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        let Some(end) = self.pos.checked_add(n).filter(|&e| e <= self.data.len()) else { return Err(Error::new("Truncated file")) };
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u16(&mut self) -> Res<u16> {
        Ok(u16_at(self.take(2)?, 0))
    }

    fn u32(&mut self) -> Res<u32> {
        Ok(u32_at(self.take(4)?, 0))
    }

    fn f32(&mut self) -> Res<f32> {
        Ok(f32_at(self.take(4)?, 0))
    }

    fn name(&mut self, end: usize, id: u16) -> Res<Vec<u8>> {
        let rest = self.data.get(self.pos..end).unwrap_or(&[]);
        let Some(stop) = rest.iter().position(|&b| b == 0) else { return Err(bad("Unterminated name", id)) };
        self.pos += stop + 1;
        Ok(rest[..stop].to_vec())
    }

    fn list(&mut self, end: usize, size: usize, id: u16) -> Res<&'a [u8]> {
        let count = self.u16()? as usize;
        self.within(end, count * size, id)
    }

    fn within(&mut self, end: usize, size: usize, id: u16) -> Res<&'a [u8]> {
        if self.pos + size > end {
            return Err(bad("Truncated chunk", id));
        }
        self.take(size)
    }

    fn chunk(&mut self, end: usize) -> Res<(u16, usize)> {
        let id = self.u16()?;
        let length = self.u32()? as usize;
        if length < CHUNK_HEADER {
            return Err(Error::new("Bad chunk length").set_err_info(format!("{id:#06x}: {length}")));
        }
        Ok((id, (self.pos - CHUNK_HEADER).saturating_add(length).min(end)))
    }

    fn colour(&mut self, end: usize, colour: &mut Colour) -> Res<()> {
        while self.pos + CHUNK_HEADER <= end {
            let (id, next) = self.chunk(end)?;
            let rgb = match id {
                COLOUR_U8 | COLOUR_U8_GAMMA if self.pos + 3 <= next => {
                    let b = self.take(3)?;
                    Some([b[0] as f32 / 255.0, b[1] as f32 / 255.0, b[2] as f32 / 255.0])
                }
                COLOUR_F32 | COLOUR_F32_GAMMA if self.pos + 12 <= next => Some([self.f32()?, self.f32()?, self.f32()?]),
                _ => None,
            };
            if let Some(rgb) = rgb {
                let slot = if matches!(id, COLOUR_U8_GAMMA | COLOUR_F32_GAMMA) { &mut colour.gamma } else { &mut colour.plain };
                slot.get_or_insert(rgb);
            }
            self.pos = next;
        }
        Ok(())
    }

    /// The first whole percentage of a chunk; a float one (0x0031) is passed over, its range not settled
    fn percent(&mut self, end: usize) -> Res<Option<u16>> {
        let mut out = None;
        while self.pos + CHUNK_HEADER <= end {
            let (id, next) = self.chunk(end)?;
            if id == PERCENT_U16 && self.pos + 2 <= next && out.is_none() {
                out = Some(self.u16()?);
            }
            self.pos = next;
        }
        Ok(out)
    }

    fn map_name(&mut self, end: usize) -> Res<Option<Vec<u8>>> {
        let mut out = None;
        while self.pos + CHUNK_HEADER <= end {
            let (id, next) = self.chunk(end)?;
            if id == MAP_NAME && out.is_none() {
                out = Some(self.name(next, id)?);
            }
            self.pos = next;
        }
        Ok(out)
    }

    fn progress(&mut self, p: &mut Parser) -> Res<()> {
        if self.pos.saturating_sub(self.taken) >= p.io.progress_interval {
            p.take_buffered(self.pos - self.taken);
            self.taken = self.pos;
            p.report_progress()?;
        }
        Ok(())
    }

    fn walk(&mut self, p: &mut Parser, end: usize, file: &mut File, name: &[u8]) -> Res<()> {
        while self.pos + CHUNK_HEADER <= end {
            self.progress(p)?;
            let (id, next) = self.chunk(end)?;
            match id {
                MAIN | EDITOR => self.walk(p, next, file, name)?,
                OBJECT => {
                    let own = self.name(next, id)?;
                    self.walk(p, next, file, &own)?;
                }
                TRIMESH => {
                    file.objects.push(Object { name: name.to_vec(), ..Object::default() });
                    self.walk(p, next, file, name)?;
                }
                VERTICES if self.pos + 2 <= next => {
                    let list = self.list(next, 12, id)?;
                    let object = file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?;
                    object.positions = list.as_chunks::<12>().0.iter().map(|v| [f32_at(v, 0), f32_at(v, 4), f32_at(v, 8)]).collect();
                }
                FACES if self.pos + 2 <= next => {
                    let list = self.list(next, 8, id)?;
                    let faces = list.as_chunks::<8>().0.iter().map(|f| [u16_at(f, 0), u16_at(f, 2), u16_at(f, 4)]).collect();
                    file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?.faces = faces;
                    // the face list's own chunks, its material groups and smoothing groups, follow its faces
                    self.walk(p, next, file, name)?;
                }
                TEX_VERTS if self.pos + 2 <= next => {
                    let list = self.list(next, 8, id)?;
                    let uvs = list.as_chunks::<8>().0.iter().map(|t| [f32_at(t, 0), f32_at(t, 4)]).collect();
                    file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?.uvs = uvs;
                }
                MESH_MATRIX => {
                    let list = self.within(next, 48, id)?;
                    let matrix = std::array::from_fn(|k| f32_at(list, 4 * k));
                    file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?.matrix = Some(matrix);
                }
                FACE_MATERIAL => {
                    let material = self.name(next, id)?;
                    let faces = self.list(next, 2, id)?.as_chunks::<2>().0.iter().map(|f| u16_at(f, 0)).collect();
                    file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?.material_groups.push((material, faces));
                }
                SMOOTHING => {
                    let object = file.objects.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?;
                    let list = self.within(next, 4 * object.faces.len(), id)?;
                    object.smoothing = list.as_chunks::<4>().0.iter().map(|g| u32_at(g, 0)).collect();
                }
                MATERIAL => {
                    file.materials.push(Material::default());
                    self.walk(p, next, file, name)?;
                }
                MATERIAL_NAME => {
                    let own = self.name(next, id)?;
                    file.materials.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?.name = own;
                }
                DIFFUSE => {
                    let material = file.materials.last_mut().ok_or_else(|| bad("Chunk outside its parent", id))?;
                    self.colour(next, &mut material.diffuse)?;
                }
                // the rest of a material is read where it is whole and passed over where not, refusing no file
                AMBIENT | SPECULAR => {
                    let mut colour = Colour::default();
                    if self.colour(next, &mut colour).is_ok()
                        && let Some(material) = file.materials.last_mut()
                    {
                        let slot = if id == AMBIENT { &mut material.ambient } else { &mut material.specular };
                        slot.plain = slot.plain.or(colour.plain);
                        slot.gamma = slot.gamma.or(colour.gamma);
                    }
                }
                TRANSPARENCY => {
                    if let Ok(percent) = self.percent(next)
                        && let Some(material) = file.materials.last_mut()
                    {
                        material.transparency = material.transparency.or(percent);
                    }
                }
                _ => {
                    if let Some(&(_, prop)) = MAPS.iter().find(|(map, _)| *map == id)
                        && let Ok(Some(file_name)) = self.map_name(next)
                        && let Some(material) = file.materials.last_mut()
                        && !material.maps.iter().any(|(p, _)| *p == prop)
                    {
                        material.maps.push((prop, file_name));
                    }
                }
            }
            self.pos = next;
        }
        Ok(())
    }
}

fn prop_real(name: &str, prop_type: PropType, values: &[Real]) -> Prop {
    let mut prop = Prop { name: name.to_string(), internal_key: name_key(name.as_bytes()), prop_type, ..Default::default() };
    prop.value_real_arr[..values.len()].copy_from_slice(values);
    prop.value_int = values[0] as i64;
    prop.flags = prop_flags::VALUE_INT | prop_flags::VALUE_REAL << (values.len() - 1);
    prop
}

fn colour_prop(name: &str, colour: Colour) -> Option<Prop> {
    colour.value().map(|c| prop_real(name, PropType::Color, &c.map(Real::from)))
}

fn material_id(ctx: &mut Context, name: &str) -> u64 {
    let fbx_id = ctx.synthetic_id_from_string(name.as_bytes());
    if ctx.find_fbx_id(fbx_id).is_none() {
        let info = ElementInfo { fbx_id, name: name.to_string(), ..Default::default() };
        let ix = ctx.push_element(&info, ElementType::Material);
        ctx.scene.materials[ix].shading_model_name = "phong".to_string();
    }
    fbx_id
}

/// One for each name, its last definition taking it
fn push_materials(ctx: &mut Context, materials: &[Material]) -> Res<()> {
    let mut defined: Vec<(String, &Material)> = Vec::new();
    for material in materials {
        let name = ctx.p.utf8(&material.name)?;
        match defined.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = material,
            None => defined.push((name, material)),
        }
    }
    for (name, material) in defined {
        let fbx_id = material_id(ctx, &name);
        let mut props: Vec<Prop> = [("AmbientColor", material.ambient), ("DiffuseColor", material.diffuse), ("SpecularColor", material.specular)]
            .into_iter()
            .filter_map(|(n, c)| colour_prop(n, c))
            .collect();
        if let Some(percent) = material.transparency {
            props.push(prop_real("TransparencyFactor", PropType::Number, &[Real::from(percent) / 100.0]));
        }
        sort_properties(&mut props);
        let Some(entry) = ctx.find_fbx_id(fbx_id) else { return crate::error::fail() };
        let ix = ctx.scene.elements[entry.element_id as usize].index;
        ctx.scene.materials[ix].element.props.props = props;

        for (prop, raw) in &material.maps {
            let file_name = ctx.p.utf8(raw)?;
            let (texture, texture_id) = ctx.push_synthetic_element(None, Some(""), ElementType::Texture);
            let t = &mut ctx.scene.textures[texture];
            t.relative_filename = file_name;
            t.raw_relative_filename = raw.clone();
            ctx.connect_op(texture_id, fbx_id, prop);
        }
    }
    Ok(())
}

fn node_material(ctx: &mut Context, used: &mut Vec<u64>, node_id: u64, fbx_id: u64) -> u32 {
    match used.iter().position(|&u| u == fbx_id) {
        Some(index) => index as u32,
        None => {
            ctx.connect_oo(fbx_id, node_id);
            used.push(fbx_id);
            used.len() as u32 - 1
        }
    }
}

fn push_object(ctx: &mut Context, object: &Object) -> Res<()> {
    let name = ctx.p.utf8(&object.name)?;
    let (node, node_id) = ctx.push_synthetic_element(None, Some(&name), ElementType::Node);
    let (mesh, mesh_id) = ctx.push_synthetic_element(None, Some(&name), ElementType::Mesh);
    let node_element_id = ctx.scene.nodes[node].element.element_id;
    ctx.tmp_node_ids.push(node_element_id);
    ctx.connect_oo(mesh_id, node_id);
    ctx.connect_oo(node_id, 0);

    // a face's material is its last group's, as the groups are read in turn, indexing the node's materials
    let mut face_material = vec![NO_INDEX; object.faces.len()];
    let mut used: Vec<u64> = Vec::new();
    for (raw, faces) in &object.material_groups {
        let material = ctx.p.utf8(raw)?;
        let fbx_id = material_id(ctx, &material);
        let index = node_material(ctx, &mut used, node_id, fbx_id);
        for &f in faces {
            if let Some(slot) = face_material.get_mut(f as usize) {
                *slot = index;
            }
        }
    }
    // a material of no colour, as Assimp gives a default one, where ufbx's clamp of a missing index would give the
    // node's first, which may be glass; an object in no group at all keeps no material
    if !used.is_empty() && face_material.contains(&NO_INDEX) {
        let fbx_id = material_id(ctx, DEFAULT_MATERIAL);
        let index = node_material(ctx, &mut used, node_id, fbx_id);
        face_material.iter_mut().filter(|m| **m == NO_INDEX).for_each(|m| *m = index);
    }

    if !ctx.opts.ignore_geometry {
        let num_faces = object.faces.len();
        let num_positions = object.positions.len();
        let mut indices = Vec::with_capacity(num_faces * 3);
        for face in &object.faces {
            for &v in face {
                let v = u32::from(v);
                indices.push(if (v as usize) < num_positions { v } else { ctx.fix_index(v, num_positions)? });
            }
        }

        let uv_indices = if object.uvs.is_empty() {
            None
        } else {
            let num_uvs = object.uvs.len();
            let mut uv_indices = indices.clone();
            for ix in &mut uv_indices {
                if *ix as usize >= num_uvs {
                    *ix = ctx.fix_index(*ix, num_uvs)?;
                }
            }
            Some(uv_indices)
        };

        let m = &mut ctx.scene.meshes[mesh];
        m.faces = (0..num_faces as u32).map(|f| Face { index_begin: 3 * f, num_indices: 3 }).collect();
        m.face_material = face_material;
        if object.smoothing.len() == num_faces && num_faces > 0 {
            m.face_smoothing = object.smoothing.iter().map(|&g| g != 0).collect();
            m.face_smoothing_groups = object.smoothing.clone();
        }
        m.vertex_position.exists = true;
        m.vertex_position.values = object.positions.iter().map(|p| Vec3::new(p[0].into(), p[1].into(), p[2].into())).collect();
        m.vertex_position.indices = indices;
        m.vertex_position.unique_per_vertex = true;
        if let Some(uv_indices) = uv_indices {
            m.vertex_uv.exists = true;
            m.vertex_uv.values = object.uvs.iter().map(|t| Vec2::new(t[0].into(), t[1].into())).collect();
            m.vertex_uv.indices = uv_indices;
            m.vertex_uv.unique_per_vertex = true;
        }
        m.mesh_matrix = object.matrix.map(|a| {
            let a = a.map(Real::from);
            Matrix { m00: a[0], m10: a[1], m20: a[2], m01: a[3], m11: a[4], m21: a[5], m02: a[6], m12: a[7], m22: a[8], m03: a[9], m13: a[10], m23: a[11] }
        });
    }
    finalize_mesh(&mut ctx.scene.meshes[mesh])
}

pub(crate) fn tds_load(ctx: &mut Context) -> Res<()> {
    let p = &mut ctx.p;
    p.pause_progress();
    while !p.io.eof {
        p.refill(p.io.data_size.saturating_mul(2).max(1), false)?;
    }
    let data = p.buffered();

    let root_info = ElementInfo { fbx_id: ctx.root_id, name: String::new(), ..Default::default() };
    let root = ctx.push_element(&root_info, ElementType::Node);
    setup_root_node(ctx, root);
    let root_element_id = ctx.scene.nodes[root].element.element_id;
    ctx.tmp_node_ids.push(root_element_id);

    let mut reader = Reader { data, pos: 0, taken: 0 };
    let mut file = File::default();
    reader.walk(&mut ctx.p, data.len(), &mut file, b"")?;
    ctx.p.take_buffered(reader.pos.min(data.len()).saturating_sub(reader.taken));

    init_file_paths(&mut ctx.p, &mut ctx.scene.metadata, &ctx.opts)?;
    push_materials(ctx, &file.materials)?;
    for object in &file.objects {
        push_object(ctx, object)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorType, WarningType};
    use crate::load::{load_memory, load_stream};
    use crate::math::transform_position;
    use std::sync::{Arc, Mutex};

    fn chunk(id: u16, body: &[u8]) -> Vec<u8> {
        [&id.to_le_bytes()[..], &((body.len() + CHUNK_HEADER) as u32).to_le_bytes(), body].concat()
    }

    fn floats(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn shorts(v: &[u16]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn named(name: &str) -> Vec<u8> {
        [name.as_bytes(), &[0]].concat()
    }

    const SQUARE: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
    const MATRIX: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 2.0, -3.0, 0.25];

    fn object(name: &str, faces: &[[u16; 3]], groups: &[(&str, &[u16])], smoothing: Option<&[u32]>, uvs: bool) -> Vec<u8> {
        let mut face_list = shorts(&[faces.len() as u16]);
        for f in faces {
            face_list.extend(shorts(&[f[0], f[1], f[2], 7]));
        }
        for (material, group) in groups {
            face_list.extend(chunk(FACE_MATERIAL, &[named(material), shorts(&[group.len() as u16]), shorts(group)].concat()));
        }
        if let Some(s) = smoothing {
            face_list.extend(chunk(SMOOTHING, &s.iter().flat_map(|g| g.to_le_bytes()).collect::<Vec<u8>>()));
        }
        let mut mesh = chunk(VERTICES, &[shorts(&[4]), floats(&SQUARE)].concat());
        if uvs {
            mesh.extend(chunk(TEX_VERTS, &[shorts(&[4]), floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.5])].concat()));
        }
        mesh.extend(chunk(MESH_MATRIX, &floats(&MATRIX)));
        mesh.extend(chunk(FACES, &face_list));
        chunk(OBJECT, &[named(name), chunk(TRIMESH, &mesh)].concat())
    }

    fn material(name: &str, chunks: &[Vec<u8>]) -> Vec<u8> {
        chunk(MATERIAL, &[chunk(MATERIAL_NAME, &named(name)), chunks.concat()].concat())
    }

    fn file(editor: &[Vec<u8>]) -> Vec<u8> {
        chunk(MAIN, &chunk(EDITOR, &editor.concat()))
    }

    fn boat() -> Vec<u8> {
        let paint = material(
            "paint",
            &[
                chunk(AMBIENT, &chunk(COLOUR_F32, &floats(&[0.1, 0.2, 0.3]))),
                chunk(DIFFUSE, &[chunk(COLOUR_F32_GAMMA, &floats(&[0.5, 0.5, 0.5])), chunk(COLOUR_U8, &[255, 0, 51]), chunk(COLOUR_U8, &[1, 2, 3])].concat()),
                chunk(SPECULAR, &chunk(COLOUR_U8_GAMMA, &[51, 102, 255])),
                chunk(TRANSPARENCY, &chunk(PERCENT_U16, &shorts(&[15]))),
                chunk(0xA200, &[chunk(PERCENT_U16, &shorts(&[100])), chunk(MAP_NAME, &named("teak.jpg"))].concat()),
                chunk(0xA230, &chunk(MAP_NAME, &named("bumps.png"))),
            ],
        );
        let glass = material("glass", &[chunk(DIFFUSE, &chunk(COLOUR_F32_GAMMA, &floats(&[0.25, 0.5, 1.0])))]);
        let hull = object("hull", &[[0, 1, 2], [1, 3, 2]], &[("paint", &[0, 1]), ("glass", &[1])], Some(&[1, 0]), true);
        let deck = object("deck", &[[0, 1, 2]], &[], None, false);
        file(&[paint, glass, hull, deck])
    }

    #[test]
    fn reads_objects_materials_and_textures() {
        let scene = load_memory(&boat(), &LoadOpts::default()).unwrap();
        assert_eq!(scene.metadata.file_format, FileFormat::Tds);
        assert_eq!((scene.settings.axes, scene.settings.unit_meters), (CoordinateAxes::RIGHT_HANDED_Z_UP, 0.0));
        assert!(scene.metadata.warnings.is_empty());
        let names: Vec<&str> = scene.nodes.iter().map(|n| n.element.name.as_str()).collect();
        assert_eq!(names, ["", "hull", "deck"]);

        let hull = &scene.nodes[1];
        let m = &scene.meshes[hull.mesh.unwrap()];
        assert_eq!(m.vertices[3], Vec3::new(1.0, 1.0, 0.0));
        assert_eq!((m.num_faces, m.num_triangles, m.vertex_indices.clone()), (2, 2, vec![0, 1, 2, 1, 3, 2]));
        assert_eq!((m.face_smoothing.clone(), m.face_smoothing_groups.clone()), (vec![true, false], vec![1, 0]));
        assert_eq!(m.vertex_uv.values[2], Vec2::new(0.0, 0.5));
        assert_eq!(m.uv_sets.len(), 1);
        let matrix = m.mesh_matrix.unwrap();
        assert_eq!((matrix.m00, matrix.m11, matrix.m03, matrix.m13, matrix.m23), (1.0, 1.0, 2.0, -3.0, 0.25));
        let materials: Vec<&str> = hull.materials.iter().map(|&i| scene.materials[i.unwrap()].element.name.as_str()).collect();
        assert_eq!(materials, ["paint", "glass"]);
        assert_eq!(m.face_material, [0, 1]);

        let deck = &scene.meshes[scene.nodes[2].mesh.unwrap()];
        assert!(deck.face_smoothing.is_empty() && !deck.vertex_uv.exists && deck.materials.is_empty());

        let paint = &scene.materials[0];
        let f = |c: [f32; 3]| Vec4::new(c[0].into(), c[1].into(), c[2].into(), 1.0);
        assert_eq!(paint.shader_type, ShaderType::FbxPhong);
        assert_eq!(paint.fbx.diffuse_color().value_vec4, f([1.0, 0.0, 51.0 / 255.0]));
        assert_eq!(paint.fbx.ambient_color().value_vec4, f([0.1, 0.2, 0.3]));
        assert_eq!(paint.fbx.specular_color().value_vec4, f([51.0 / 255.0, 102.0 / 255.0, 1.0]));
        assert_eq!(paint.fbx.transparency_factor().value_real(), 0.15);
        let texture = |map: &MaterialMap| scene.textures[map.texture.unwrap()].relative_filename.as_str();
        assert_eq!((texture(paint.fbx.diffuse_color()), texture(paint.fbx.bump())), ("teak.jpg", "bumps.png"));
        assert_eq!(scene.materials[1].fbx.diffuse_color().value_vec4, f([0.25, 0.5, 1.0]));
    }

    #[test]
    fn found_by_content_or_by_name_after_ufbxs_own_formats() {
        let by_name = LoadOpts { filename: "boat.3DS".into(), no_format_from_content: true, ..LoadOpts::default() };
        assert_eq!(load_memory(&boat(), &by_name).unwrap().metadata.file_format, FileFormat::Tds);
        let obj = b"MM is a comment line longer than the first look of 32 bytes\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        match load_memory(obj, &LoadOpts::default()) {
            Ok(scene) => assert_eq!(scene.metadata.file_format, FileFormat::Obj),
            Err(e) => assert_eq!(e.info, "format-obj"),
        }
        let e = load_memory(b"MM\x06", &LoadOpts::default()).unwrap_err();
        assert_eq!(e.kind, ErrorType::UnrecognizedFileFormat);
    }

    #[test]
    fn a_damaged_file_fails_at_the_chunk_that_is() {
        let fails = |bytes: &[u8]| {
            let e = load_memory(bytes, &LoadOpts::default()).unwrap_err();
            (e.description, e.info)
        };
        let whole = boat();
        let at = |id: u16| whole.windows(2).position(|w| w == id.to_le_bytes()).unwrap();
        let mut short = whole.clone();
        short[at(TRIMESH) + 2..at(TRIMESH) + 6].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(fails(&short), ("Bad chunk length".into(), "0x4100: 3".into()));
        let mut lying = whole.clone();
        lying[at(VERTICES) + 6] = 200;
        assert_eq!(fails(&lying), ("Truncated chunk".into(), "0x4110".into()));
        let mut lying = whole.clone();
        lying[at(SMOOTHING) + 2] = 9;
        assert_eq!(fails(&lying), ("Truncated chunk".into(), "0x4150".into()));
        let mut lying = whole.clone();
        lying[at(TEX_VERTS) + 6] = 200;
        assert_eq!(fails(&lying), ("Truncated chunk".into(), "0x4140".into()));
        let mut short = whole.clone();
        short[at(MESH_MATRIX) + 2..at(MESH_MATRIX) + 6].copy_from_slice(&(6u32 + 44).to_le_bytes());
        assert_eq!(fails(&short), ("Truncated chunk".into(), "0x4160".into()));
        assert_eq!(fails(&file(&[chunk(OBJECT, b"no end")])), ("Unterminated name".into(), "0x4000".into()));
        assert_eq!(fails(&file(&[chunk(VERTICES, &shorts(&[0]))])), ("Chunk outside its parent".into(), "0x4110".into()));
        // cut ten bytes into the second object's vertices, past its count
        let second = whole.windows(2).enumerate().filter(|(_, w)| *w == VERTICES.to_le_bytes()).nth(1).unwrap().0;
        assert_eq!(fails(&whole[..second + 6 + 2 + 10]), ("Truncated chunk".into(), "0x4110".into()));
    }

    #[test]
    fn the_rest_of_a_damaged_material_is_passed_over() {
        let bad_length = [&SPECULAR.to_le_bytes()[..], &12u32.to_le_bytes(), &COLOUR_U8.to_le_bytes(), &3u32.to_le_bytes()].concat();
        let paint = material(
            "paint",
            &[
                chunk(DIFFUSE, &chunk(COLOUR_U8, &[255, 0, 0])),
                bad_length,
                chunk(0xA200, &chunk(MAP_NAME, b"no end")),
                chunk(AMBIENT, &chunk(COLOUR_U8, &[0, 255, 0])),
            ],
        );
        let stray = chunk(TRANSPARENCY, &chunk(PERCENT_U16, &shorts(&[50])));
        let scene = load_memory(&file(&[stray, paint, object("hull", &[[0, 1, 2]], &[("paint", &[0])], None, false)]), &LoadOpts::default()).unwrap();
        let fbx = &scene.materials[0].fbx;
        assert_eq!((fbx.diffuse_color().value_vec4, fbx.ambient_color().value_vec4), (Vec4::new(1.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 1.0, 0.0, 1.0)));
        assert!(!fbx.specular_color().has_value && fbx.diffuse_color().texture.is_none() && scene.textures.is_empty());
    }

    #[test]
    fn a_corner_past_the_vertices_is_clamped_left_empty_or_refused() {
        let bytes = file(&[object("hull", &[[0, 1, 9]], &[], None, true)]);
        let scene = load_memory(&bytes, &LoadOpts::default()).unwrap();
        let m = &scene.meshes[0];
        assert_eq!((m.vertex_indices.clone(), m.vertex_uv.indices.clone()), (vec![0, 1, 3], vec![0, 1, 3]));
        assert!(scene.metadata.warnings.iter().any(|w| w.kind == WarningType::IndexClamped));
        let empty_corner = LoadOpts { index_error_handling: IndexErrorHandling::NoIndex, ..LoadOpts::default() };
        let scene = load_memory(&bytes, &empty_corner).unwrap();
        assert_eq!(scene.meshes[0].vertex_indices, [0, 1, NO_INDEX]);
        assert!(scene.metadata.warnings.is_empty());
        let abort = LoadOpts { index_error_handling: IndexErrorHandling::AbortLoading, ..LoadOpts::default() };
        let e = load_memory(&bytes, &abort).unwrap_err();
        assert_eq!((e.description.as_str(), e.info.as_str()), ("Bad index", "9 (max 3)"));
        // triangles and no vertex to clamp to, as an .obj with faces and no vertices; left empty, every corner is
        let mut empty = bytes.clone();
        let at = empty.windows(2).position(|w| w == VERTICES.to_le_bytes()).unwrap();
        empty[at..at + 2].copy_from_slice(&0x4111u16.to_le_bytes());
        assert_eq!(load_memory(&empty, &LoadOpts::default()).unwrap_err().description, "Failed to load");
        let m = &load_memory(&empty, &empty_corner).unwrap().meshes[0];
        assert_eq!((m.num_vertices, m.vertex_indices.clone()), (0, vec![NO_INDEX; 3]));
    }

    #[test]
    fn every_triangle_mesh_is_a_node_one_without_faces_too() {
        let faceless = chunk(OBJECT, &[named("rail"), chunk(TRIMESH, &chunk(VERTICES, &[shorts(&[4]), floats(&SQUARE)].concat()))].concat());
        let bare = chunk(OBJECT, &[named("bare"), chunk(TRIMESH, &[])].concat());
        let lamp = chunk(OBJECT, &[named("lamp"), chunk(0x4600, &floats(&[0.0, 0.0, 9.0]))].concat());
        let scene = load_memory(&file(&[faceless, lamp, bare]), &LoadOpts::default()).unwrap();
        let nodes: Vec<(&str, Option<usize>)> = scene.nodes.iter().map(|n| (n.element.name.as_str(), n.mesh.map(|m| scene.meshes[m].num_vertices))).collect();
        assert_eq!(nodes, [("", None), ("rail", Some(4)), ("bare", Some(0))]);
        assert!(scene.meshes.iter().all(|m| m.num_faces == 0));
        let empty = load_memory(&file(&[]), &LoadOpts::default()).unwrap();
        assert_eq!((empty.nodes.len(), empty.meshes.len()), (1, 0));
    }

    #[test]
    fn a_material_is_one_per_name_its_last_definition_taking_it() {
        let red = material("paint", &[chunk(DIFFUSE, &chunk(COLOUR_U8, &[255, 0, 0]))]);
        let blue = material("paint", &[chunk(DIFFUSE, &chunk(COLOUR_U8, &[0, 0, 255]))]);
        let hull = object("hull", &[[0, 1, 2], [1, 3, 2], [0, 3, 2]], &[("missing", &[0]), ("paint", &[1, 5])], None, false);
        let deck = object("deck", &[[0, 1, 2]], &[], None, false);
        let scene = load_memory(&file(&[red, blue, hull, deck]), &LoadOpts::default()).unwrap();
        let names: Vec<&str> = scene.materials.iter().map(|m| m.element.name.as_str()).collect();
        assert_eq!(names, ["paint", "missing", "DefaultMaterial"]);
        assert_eq!(scene.materials[0].fbx.diffuse_color().value_vec4, Vec4::new(0.0, 0.0, 1.0, 1.0));
        assert!(!scene.materials[1].fbx.diffuse_color().has_value);
        assert!(!scene.materials[2].fbx.diffuse_color().has_value);
        assert_eq!(scene.meshes[0].face_material, [0, 1, 2]);
        let deck = scene.nodes.iter().find(|n| n.element.name == "deck").unwrap();
        assert!(deck.materials.is_empty());
    }

    #[test]
    fn read_through_a_stream_with_progress_and_cancels() {
        let bytes = boat();
        let memory = load_memory(&bytes, &LoadOpts::default()).unwrap();
        let small = LoadOpts { read_buffer_size: 32, ..LoadOpts::default() };
        let streamed = load_stream(&mut std::io::Cursor::new(&bytes), &small).unwrap();
        assert_eq!(streamed.meshes, memory.meshes);
        assert_eq!(streamed.materials, memory.materials);

        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = calls.clone();
        let cancel_at = Arc::new(Mutex::new(usize::MAX));
        let stop = cancel_at.clone();
        let cb: Arc<ProgressFn> = Arc::new(move |p: &Progress| {
            let mut v = seen.lock().unwrap();
            v.push((p.bytes_read, p.bytes_total));
            if v.len() >= *stop.lock().unwrap() { ProgressResult::Cancel } else { ProgressResult::Continue }
        });
        let opts = LoadOpts { progress_cb: Some(cb), progress_interval_hint: 64, ..LoadOpts::default() };
        load_memory(&bytes, &opts).unwrap();
        let v = calls.lock().unwrap().clone();
        assert!(v.len() >= 3 && v.windows(2).all(|w| w[0].0 < w[1].0) && v.iter().all(|&(read, total)| read <= total && total == bytes.len() as u64));
        calls.lock().unwrap().clear();
        *cancel_at.lock().unwrap() = 2;
        assert_eq!(load_memory(&bytes, &opts).unwrap_err().kind, ErrorType::Cancelled);

        struct Failing(usize);
        impl std::io::Read for Failing {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.0 == 0 {
                    return Err(std::io::Error::other("unplugged"));
                }
                let n = buf.len().min(self.0);
                buf[..n].fill(0x4D);
                self.0 -= n;
                Ok(n)
            }
        }
        assert_eq!(load_stream(&mut Failing(40), &small).unwrap_err().kind, ErrorType::Io);
    }

    #[test]
    fn its_axes_and_unit_are_converted_as_ufbx_converts_others() {
        let opts = LoadOpts { tds_unit_meters: 0.01, target_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, target_unit_meters: 1.0, ..LoadOpts::default() };
        let scene = load_memory(&boat(), &opts).unwrap();
        let (node, mesh) = (&scene.nodes[1], &scene.meshes[0]);
        let p = transform_position(&node.geometry_to_world, mesh.vertices[2]);
        assert!((p.x - 0.0).abs() < 1e-12 && (p.y - 0.0).abs() < 1e-12 && (p.z + 0.01).abs() < 1e-12, "{p:?}");
        let told = LoadOpts { tds_axes: CoordinateAxes::RIGHT_HANDED_Y_UP, ..LoadOpts::default() };
        assert_eq!(load_memory(&boat(), &told).unwrap().settings.axes, CoordinateAxes::RIGHT_HANDED_Y_UP);
    }
}
