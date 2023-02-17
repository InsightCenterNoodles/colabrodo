use colabrodo_common::components::*;
use colabrodo_common::nooid::NooID;

pub use colabrodo_common::components::{
    AttributeSemantic, BufferRepresentation, BufferState, BufferViewType,
    GeometryIndex, MethodArg, MethodState, PrimitiveType, SignalState,
};

pub type ClientRenderRepresentation = RenderRepresentation<NooID>;

pub type ClientEntityRepresentation = EntityRepresentation<NooID>;

pub type ClientGeometryAttribute = GeometryAttribute<NooID>;

pub type ClientGeometryIndex = GeometryIndex<NooID>;

pub type ClientGeometryPatch = GeometryPatch<NooID, NooID>;

pub type ClientGeometryState = GeometryState<NooID, NooID>;

pub type ClientBufferViewState = BufferViewState<NooID>;

pub type ClientTextureRef = TextureRef<NooID>;

pub type ClientPBRInfo = PBRInfo<NooID>;

pub type ClientImageState = ImageState<NooID>;

pub type ClientTextureState = TextureState<NooID, NooID>;
