use colabrodo_common::nooid::NooID;
use colabrodo_common::server_communication::MessageSignalInvoke;
use colabrodo_common::{components::*, server_communication::DocumentUpdate};

pub use colabrodo_common::components::{
    AttributeSemantic, BufferRepresentation, BufferState, BufferViewType,
    GeometryIndex, MethodArg, MethodState, PrimitiveType, SamplerState,
    SignalState,
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

pub type ClientMaterialState = MaterialState<NooID>;
pub type ClientMaterialUpdate = MaterialStateUpdatable<NooID>;

pub type ClientTableState = TableState<NooID, NooID>;
pub type ClientTableUpdate = TableStateUpdatable<NooID, NooID>;

pub type ClientPlotUpdate = PlotStateUpdatable<NooID, NooID, NooID>;
pub type ClientPlotState = PlotState<NooID, NooID, NooID>;

pub type ClientEntityUpdate =
    EntityStateUpdatable<NooID, NooID, NooID, NooID, NooID, NooID, NooID>;
pub type ClientEntityState =
    EntityState<NooID, NooID, NooID, NooID, NooID, NooID, NooID>;

pub type ClientDocumentUpdate = DocumentUpdate<NooID, NooID>;

pub type ClientMessageSignalInvoke = MessageSignalInvoke<NooID, NooID, NooID>;
