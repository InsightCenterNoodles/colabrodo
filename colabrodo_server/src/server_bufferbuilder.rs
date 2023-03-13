//! Tools to help pack mesh buffers. See [VertexSource] to see how to use this module.

use std::mem::size_of;

use crate::{server_messages::*, server_state::ServerState};
use colabrodo_common::types::*;

use bytemuck::{self, try_cast_slice, Zeroable};

use bytemuck::Pod;
use thiserror::Error;

/// A minimal vertex, with only position and normal
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Copy, Zeroable, Pod)]
pub struct VertexMinimal {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// A vertex with texture support
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Copy, Zeroable, Pod)]
pub struct VertexTexture {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [u16; 2],
}

/// A vertex type with support for all attributes
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Copy, Zeroable, Pod)]
pub struct VertexFull {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub texture: [u16; 2],
    pub color: [u8; 4],
}

// =============================================================================

pub trait Vertex: bytemuck::NoUninit {
    fn get_structure() -> &'static [BuilderAttrib];
}

pub struct BuilderAttrib {
    pub semantic: AttributeSemantic,
    pub offset: Option<u32>,
    pub format: Format,
    pub normalized: Option<bool>,
}

// =============================================================================

impl Vertex for VertexMinimal {
    fn get_structure() -> &'static [BuilderAttrib] {
        static RET: [BuilderAttrib; 2] = [
            BuilderAttrib {
                semantic: AttributeSemantic::Position,
                offset: Some(0),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Normal,
                offset: Some(12),
                format: Format::VEC3,
                normalized: Some(false),
            },
        ];
        &RET
    }
}

// =============================================================================

impl Vertex for VertexTexture {
    fn get_structure() -> &'static [BuilderAttrib] {
        static RET: [BuilderAttrib; 3] = [
            BuilderAttrib {
                semantic: AttributeSemantic::Position,
                offset: Some(0),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Normal,
                offset: Some(12),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Texture,
                offset: Some(24),
                format: Format::U16VEC2,
                normalized: Some(true),
            },
        ];
        &RET
    }
}

// =============================================================================

impl Vertex for VertexFull {
    fn get_structure() -> &'static [BuilderAttrib] {
        static RET: [BuilderAttrib; 5] = [
            BuilderAttrib {
                semantic: AttributeSemantic::Position,
                offset: Some(0),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Normal,
                offset: Some(12),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Tangent,
                offset: Some(24),
                format: Format::VEC3,
                normalized: Some(false),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Texture,
                offset: Some(36),
                format: Format::U16VEC2,
                normalized: Some(true),
            },
            BuilderAttrib {
                semantic: AttributeSemantic::Color,
                offset: Some(40),
                format: Format::U8VEC4,
                normalized: Some(true),
            },
        ];
        &RET
    }
}

// =============================================================================

#[derive(Error, Debug)]
pub enum BufferBuildError {
    #[error("Internal error")]
    InternalError,
}

// =============================================================================

/// How should the buffer be represented? Either by bytes inline, or by some URL
pub enum BufferRepresentation {
    Bytes(Vec<u8>),
    Url(String),
}

// =============================================================================

/// The type of index for this mesh
#[derive(Debug, Clone)]
pub enum IndexType<'a> {
    UnindexedPoints,
    UnindexedLines,
    UnindexedTriangles,
    Points(&'a [u32]),
    Lines(&'a [[u32; 2]]),
    Triangles(&'a [[u32; 3]]),
}

impl<'a> IndexType<'a> {
    fn total_u32_count(&self) -> u32 {
        (match self {
            IndexType::Points(x) => x.len(),
            IndexType::Lines(x) => x.len() * 2,
            IndexType::Triangles(x) => x.len() * 3,
            _ => 0,
        }) as u32
    }

    fn total_byte_count(&self) -> usize {
        match self {
            IndexType::Points(x) => x.len() * 4,
            IndexType::Lines(x) => x.len() * 2 * 4,
            IndexType::Triangles(x) => x.len() * 3 * 4,
            _ => 0,
        }
    }
}

/// Compressed mesh representation
pub struct PackResult {
    pub bytes: Vec<u8>,
    pub vertex_region_size: u64,
    pub index_region_size: u64,
}

/// Specification of mesh info to be packed.
///
/// Users can set an optional name, a list of verticies to pack, and how to index those verticies.
///
/// To use, first instantiate one of these types with your vertex information.
/// You can then pack the bytes to get the binary representation. After, you can then build NOODLES state. This split approach lets you choose where the bytes of the mesh should actually be stored.
#[derive(Debug)]
pub struct VertexSource<'a, V>
where
    V: Vertex,
{
    pub name: Option<String>,
    pub vertex: &'a [V],
    pub index: IndexType<'a>,
}

impl<'a, V> VertexSource<'a, V>
where
    V: Vertex,
{
    /// Compress the vertex and index information into a byte array
    pub fn pack_bytes(&self) -> Result<PackResult, BufferBuildError> {
        let mut bytes: Vec<u8> = Vec::new();

        let vertex_bytes = try_cast_slice(self.vertex)
            .map_err(|_| BufferBuildError::InternalError)?;

        let i_total_bytes = self.index.total_byte_count();

        bytes.reserve(vertex_bytes.len() + i_total_bytes);

        bytes.extend_from_slice(vertex_bytes);

        fn try_cast<T: Pod>(t: &[T]) -> Result<&[u8], BufferBuildError> {
            try_cast_slice(t).map_err(|_| BufferBuildError::InternalError)
        }

        let blank = Vec::<u8>::new();

        let slice = match self.index {
            IndexType::Points(x) => try_cast(x)?,
            IndexType::Lines(x) => try_cast(x)?,
            IndexType::Triangles(x) => try_cast(x)?,
            _ => blank.as_slice(),
        };

        bytes.extend_from_slice(slice);

        Ok(PackResult {
            bytes,
            vertex_region_size: vertex_bytes.len() as u64,
            index_region_size: i_total_bytes as u64,
        })
    }

    /// Build partial NOODLES state for this mesh.
    ///
    /// Lets the user choose how to handle the structure of the patch.
    /// Make sure to pass in the proper disposition of the mesh bytes.
    pub fn build_states(
        &self,
        server_state: &mut ServerState,
        representation: BufferRepresentation,
    ) -> Result<IntermediateGeometryPatch, BufferBuildError> {
        let v_byte_size = size_of::<V>();

        let vertex_bytes = (v_byte_size * self.vertex.len()) as u64;

        let i_total_bytes = self.index.total_byte_count() as u64;

        let total_bytes = vertex_bytes + i_total_bytes;

        let mut bytes_source = None;
        let mut url_source = None;

        match representation {
            BufferRepresentation::Bytes(x) => {
                bytes_source = Some(ByteBuff::new(x))
            }
            BufferRepresentation::Url(x) => {
                url_source = Some(x.parse().unwrap())
            }
        }

        let buffer = server_state.buffers.new_component(BufferState {
            name: self.name.as_ref().map(|x| format!("{x}_buffer")),
            size: total_bytes,
            inline_bytes: bytes_source,
            uri_bytes: url_source,
        });

        let vertex_view =
            server_state
                .buffer_views
                .new_component(ServerBufferViewState {
                    name: None,
                    source_buffer: buffer.clone(),
                    view_type: BufferViewType::Geometry,
                    offset: 0,
                    length: vertex_bytes,
                });

        let mut ret = IntermediateGeometryPatch {
            attributes: Vec::default(),
            vertex_count: self.vertex.len() as u64,
            indices: None,
            patch_type: match self.index {
                IndexType::UnindexedPoints => PrimitiveType::Points,
                IndexType::UnindexedLines => PrimitiveType::Lines,
                IndexType::UnindexedTriangles => PrimitiveType::Triangles,
                IndexType::Points(_) => PrimitiveType::Points,
                IndexType::Lines(_) => PrimitiveType::Lines,
                IndexType::Triangles(_) => PrimitiveType::Triangles,
            },
        };

        // record vertex offsets
        for attrib in V::get_structure() {
            ret.attributes.push(ServerGeometryAttribute {
                view: vertex_view.clone(),
                semantic: attrib.semantic,
                channel: None,
                offset: attrib.offset,
                stride: Some(v_byte_size as u32),
                format: attrib.format,
                minimum_value: None,
                maximum_value: None,
                normalized: attrib.normalized,
            });
        }

        // add in indicies
        match self.index {
            IndexType::Points(_)
            | IndexType::Lines(_)
            | IndexType::Triangles(_) => {
                let view = server_state.buffer_views.new_component(
                    ServerBufferViewState {
                        name: None,
                        source_buffer: buffer,
                        view_type: BufferViewType::Geometry,
                        offset: vertex_bytes,
                        length: i_total_bytes,
                    },
                );

                ret.indices = Some(GeometryIndex {
                    view,
                    count: self.index.total_u32_count(),
                    offset: None,
                    stride: None,
                    format: Format::U32,
                });
            }
            _ => (),
        }

        Ok(ret)
    }

    /// Build the full geometry state.
    pub fn build_geometry(
        &self,
        server_state: &mut ServerState,
        representation: BufferRepresentation,
        material: MaterialReference,
    ) -> Result<GeometryReference, BufferBuildError> {
        let intermediate = self.build_states(server_state, representation)?;

        let patch = ServerGeometryPatch {
            attributes: intermediate.attributes,
            vertex_count: intermediate.vertex_count,
            indices: intermediate.indices,
            patch_type: intermediate.patch_type,
            material,
        };

        Ok(server_state.geometries.new_component(ServerGeometryState {
            name: self.name.clone(),
            patches: vec![patch],
        }))
    }
}

#[derive(Debug)]
pub struct IntermediateGeometryPatch {
    pub attributes: Vec<ServerGeometryAttribute>,
    pub vertex_count: u64,
    pub indices: Option<ServerGeometryIndex>,
    pub patch_type: PrimitiveType,
}

#[cfg(test)]
mod tests {
    use std::mem;

    use colabrodo_common::nooid::{BufferID, NooID};

    use crate::server_state::*;

    use super::*;

    #[test]
    fn vertex_sizes() {
        assert_eq!(mem::size_of::<VertexMinimal>(), 2 * 3 * 4);
        assert_eq!(
            mem::size_of::<VertexTexture>(),
            (3 * 4) + (3 * 4) + (2 * 2)
        );

        assert_eq!(
            mem::size_of::<VertexFull>(),
            (3 * 4) + (3 * 4) + (3 * 4) + (2 * 2) + 4
        );
    }

    #[test]
    fn byte_pack() {
        let verts = vec![
            VertexTexture {
                position: [1.0, 2.0, 3.0],
                normal: [0.0, 1.0, 0.0],
                texture: [8234, 512],
            },
            VertexTexture {
                position: [4.0, 1.0, 2.0],
                normal: [1.0, 0.0, 0.0],
                texture: [27, 743],
            },
            VertexTexture {
                position: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                texture: [10, 900],
            },
        ];

        let index_list = vec![[0, 1, 2]];
        let index = IndexType::Triangles(index_list.as_slice());

        let source = VertexSource {
            name: None,
            vertex: verts.as_slice(),
            index,
        };

        let result = source.pack_bytes().unwrap();

        assert_eq!(
            result.bytes.len(),
            (mem::size_of::<VertexTexture>() * 3) + (3 * 4)
        );

        let v_slice = &result.bytes[0..(result.vertex_region_size as usize)];
        let i_slice = &result.bytes[(result.vertex_region_size as usize)..];

        let reinterp_v = bytemuck::cast_slice::<u8, VertexTexture>(v_slice);
        let retnterp_i = bytemuck::cast_slice::<u8, [u32; 3]>(i_slice);

        assert_eq!(reinterp_v, verts);
        assert_eq!(retnterp_i, index_list);
    }

    #[test]
    fn common_pack() {
        let state = ServerState::new();

        let verts = vec![
            VertexTexture {
                position: [1.0, 2.0, 3.0],
                normal: [0.0, 1.0, 0.0],
                texture: [8234, 512],
            },
            VertexTexture {
                position: [4.0, 1.0, 2.0],
                normal: [1.0, 0.0, 0.0],
                texture: [27, 743],
            },
            VertexTexture {
                position: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                texture: [10, 900],
            },
        ];

        let index_list = vec![[0, 1, 2]];
        let index = IndexType::Triangles(index_list.as_slice());

        let source = VertexSource {
            name: None,
            vertex: verts.as_slice(),
            index: index.clone(),
        };

        let mut lock = state.lock().unwrap();

        let packed = source.pack_bytes().unwrap();

        let _result = source
            .build_states(&mut lock, BufferRepresentation::Bytes(packed.bytes))
            .unwrap();

        lock.buffers.inspect(BufferID(NooID::new(0, 0)), |f| {
            let bytes = f.inline_bytes.as_ref().unwrap().bytes();

            let pack_res = source.pack_bytes().unwrap();

            assert_eq!(bytes, pack_res.bytes);
        });

        //println!("{result:?}");
    }
}
