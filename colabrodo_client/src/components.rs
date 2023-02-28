use colabrodo_common::nooid::NooID;
use colabrodo_common::server_communication::MessageSignalInvoke;
use colabrodo_common::{components::*, server_communication::DocumentUpdate};

pub use colabrodo_common::components::{
    AttributeSemantic, BufferRepresentation, BufferState, BufferViewType,
    GeometryIndex, LightState, MethodArg, MethodState, PrimitiveType,
    SamplerState, SignalState,
};

/// Trait to allow extracting names from certain components
pub trait NamedComponent {
    fn name(&self) -> Option<&String>;
}

impl NamedComponent for MethodState {
    fn name(&self) -> Option<&String> {
        Some(&self.name)
    }
}

impl NamedComponent for SignalState {
    fn name(&self) -> Option<&String> {
        Some(&self.name)
    }
}

impl NamedComponent for BufferState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

impl NamedComponent for SamplerState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

impl NamedComponent for LightState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientRenderRepresentation = RenderRepresentation<NooID, NooID>;

pub type ClientEntityRepresentation = EntityRepresentation<NooID, NooID>;

pub type ClientGeometryAttribute = GeometryAttribute<NooID>;

pub type ClientGeometryIndex = GeometryIndex<NooID>;

pub type ClientGeometryPatch = GeometryPatch<NooID, NooID>;

pub type ClientGeometryState = GeometryState<NooID, NooID>;

impl NamedComponent for ClientGeometryState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientBufferViewState = BufferViewState<NooID>;

impl NamedComponent for ClientBufferViewState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTextureRef = TextureRef<NooID>;

pub type ClientPBRInfo = PBRInfo<NooID>;

pub type ClientImageState = ImageState<NooID>;

impl NamedComponent for ClientImageState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTextureState = TextureState<NooID, NooID>;

impl NamedComponent for ClientTextureState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientMaterialState = MaterialState<NooID>;
pub type ClientMaterialUpdate = MaterialStateUpdatable<NooID>;

impl NamedComponent for ClientMaterialState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTableState = TableState<NooID, NooID>;
pub type ClientTableUpdate = TableStateUpdatable<NooID, NooID>;

impl NamedComponent for ClientTableState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientPlotUpdate = PlotStateUpdatable<NooID, NooID, NooID>;
pub type ClientPlotState = PlotState<NooID, NooID, NooID>;

impl NamedComponent for ClientPlotState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientEntityUpdate = EntityStateUpdatable<
    NooID,
    NooID,
    NooID,
    NooID,
    NooID,
    NooID,
    NooID,
    NooID,
>;
pub type ClientEntityState =
    EntityState<NooID, NooID, NooID, NooID, NooID, NooID, NooID, NooID>;

impl NamedComponent for ClientEntityState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientDocumentUpdate = DocumentUpdate<NooID, NooID>;

pub type ClientMessageSignalInvoke = MessageSignalInvoke<NooID, NooID, NooID>;
