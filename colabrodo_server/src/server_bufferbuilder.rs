use crate::{server_messages::*, server_state::ServerState};
use colabrodo_common::types::*;

#[derive(Debug, Default)]
pub struct VertexSource {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,

    pub textures: Vec<[u16; 2]>,
    pub colors: Vec<[u8; 4]>,

    pub triangles: Vec<[u32; 3]>,
}

trait ToBytes {
    fn bytes_to(&self, out: &mut Vec<u8>);
}

impl ToBytes for f32 {
    fn bytes_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

impl ToBytes for u32 {
    fn bytes_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

impl ToBytes for u16 {
    fn bytes_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

fn write_bytes<T, const N: usize>(src: &[T; N], out: &mut Vec<u8>)
where
    //T: ToBytes<ByteType = &[u8]>,
    T: ToBytes,
{
    for i in src {
        i.bytes_to(out);
    }
}

pub struct IntermediateGeometryPatch {
    pub attributes: Vec<GeometryAttribute>,
    pub vertex_count: u64,
    pub indices: Option<GeometryIndex>,
    pub patch_type: PrimitiveType,
}

pub fn create_mesh_with<F>(
    server_state: &mut ServerState,
    source: VertexSource,
    register_bytes: F,
) -> IntermediateGeometryPatch
where
    F: FnOnce(Vec<u8>) -> crate::server_messages::BufferRepresentation,
{
    let v_count = source.positions.len();

    let i_count = source.triangles.len();

    let has_n = source.normals.len() >= v_count;
    let has_t = source.textures.len() >= v_count;
    let has_c = source.colors.len() >= v_count;

    let mut v_byte_size = 3 * 4;

    if has_n {
        v_byte_size += 3 * 4;
    }

    if has_t {
        v_byte_size += 4;
    }

    if has_c {
        v_byte_size += 4;
    }

    let v_total_bytes = v_byte_size * v_count;

    let i_total_bytes = 4 * 3 * source.triangles.len();

    let mut bytes: Vec<u8> = Vec::new();

    bytes.reserve(v_total_bytes + i_total_bytes);

    // this is not efficient, but it works! so there is that.
    // we also use a few unsafe functions, but due to the checks on length
    // things should be ok

    for i in 0..v_count {
        unsafe {
            let p = source.positions.get_unchecked(i);
            write_bytes(p, &mut bytes);
        }

        if has_n {
            unsafe {
                let n = source.normals.get_unchecked(i);
                write_bytes(n, &mut bytes);
            }
        }

        if has_t {
            unsafe {
                let t = source.textures.get_unchecked(i);
                write_bytes(t, &mut bytes);
            }
        }

        if has_c {
            unsafe {
                let c = source.colors.get_unchecked(i);
                bytes.extend_from_slice(c);
            }
        }
    }

    for index in source.triangles {
        write_bytes(&index, &mut bytes);
    }

    let buffer = server_state.buffers.new_component(BufferState {
        name: Some(format!("{}_buffer", source.name)),
        size: bytes.len() as u64,
        representation: register_bytes(bytes),
    });

    let vertex_view =
        server_state.buffer_views.new_component(BufferViewState {
            name: None,
            source_buffer: buffer.clone(),
            view_type: BufferViewType::Geometry,
            offset: 0,
            length: (v_byte_size * v_count) as u64,
        });

    let mut ret = IntermediateGeometryPatch {
        attributes: Vec::default(),
        vertex_count: v_count as u64,
        indices: None,
        patch_type: PrimitiveType::Triangles,
    };

    let mut cursor = 3 * 4;

    {
        // add in position
        ret.attributes.push(GeometryAttribute {
            view: vertex_view.clone(),
            semantic: AttributeSemantic::Position,
            channel: None,
            offset: Some(0),
            stride: Some(v_byte_size as u32),
            format: Format::VEC3,
            minimum_value: None,
            maximum_value: None,
            normalized: Some(false),
        })
    }

    if has_n {
        let normal_offset = cursor;
        cursor += 3 * 4;

        ret.attributes.push(GeometryAttribute {
            view: vertex_view.clone(),
            semantic: AttributeSemantic::Normal,
            channel: None,
            offset: Some(normal_offset),
            stride: Some(v_byte_size as u32),
            format: Format::VEC3,
            minimum_value: None,
            maximum_value: None,
            normalized: Some(false),
        })
    }

    if has_t {
        let texture_offset = cursor;
        cursor += 4;

        ret.attributes.push(GeometryAttribute {
            view: vertex_view.clone(),
            semantic: AttributeSemantic::Texture,
            channel: None,
            offset: Some(texture_offset),
            stride: Some(v_byte_size as u32),
            format: Format::U16VEC2,
            minimum_value: None,
            maximum_value: None,
            normalized: Some(true),
        })
    }

    if has_c {
        let color_offset = cursor;
        //cursor += 1;

        ret.attributes.push(GeometryAttribute {
            view: vertex_view,
            semantic: AttributeSemantic::Color,
            channel: None,
            offset: Some(color_offset),
            stride: Some(v_byte_size as u32),
            format: Format::U8VEC4,
            minimum_value: None,
            maximum_value: None,
            normalized: Some(true),
        })
    }

    // add in indicies
    if i_count != 0 {
        let view = server_state.buffer_views.new_component(BufferViewState {
            name: None,
            source_buffer: buffer,
            view_type: BufferViewType::Geometry,
            offset: v_total_bytes as u64,
            length: i_total_bytes as u64,
        });

        ret.indices = Some(GeometryIndex {
            view,
            count: (i_count * 3) as u32,
            offset: None,
            stride: None,
            format: Format::U32,
        });
    }

    ret
}

pub fn create_mesh(
    server_state: &mut ServerState,
    source: VertexSource,
) -> IntermediateGeometryPatch {
    create_mesh_with(server_state, source, |data| {
        crate::server_messages::BufferRepresentation::Inline(ByteBuff {
            bytes: data,
        })
    })
}
