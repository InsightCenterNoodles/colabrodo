use colabrodo_common::nooid::*;
use colabrodo_common::server_communication::MessageSignalInvoke;
use colabrodo_common::{components::*, server_communication::DocumentUpdate};

pub use colabrodo_common::components::{
    AttributeSemantic, BufferState, BufferViewType, GeometryIndex, LightState,
    MethodArg, MethodState, PrimitiveType, SamplerState, SignalState,
};

pub type ClientMethodState = MethodState<u64>;
pub type ClientSignalState = SignalState<u64>;

/// Trait to allow extracting names from certain components
pub trait NamedComponent {
    fn name(&self) -> Option<&String>;
}

impl NamedComponent for ClientMethodState {
    fn name(&self) -> Option<&String> {
        Some(&self.name)
    }
}

impl NamedComponent for ClientSignalState {
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

pub type ClientRenderRepresentation =
    RenderRepresentation<GeometryID, BufferViewID>;

pub type ClientEntityRepresentation =
    EntityRepresentation<GeometryID, BufferViewID>;

pub type ClientGeometryAttribute = GeometryAttribute<BufferViewID>;

pub type ClientGeometryIndex = GeometryIndex<BufferViewID>;

pub type ClientGeometryPatch = GeometryPatch<BufferViewID, MaterialID>;

pub type ClientGeometryState = GeometryState<BufferViewID, MaterialID>;

impl NamedComponent for ClientGeometryState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientBufferViewState = BufferViewState<BufferID>;

impl NamedComponent for ClientBufferViewState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTextureRef = TextureRef<TextureID>;

pub type ClientPBRInfo = PBRInfo<TextureID>;

pub type ClientImageState = ImageState<BufferViewID>;

impl NamedComponent for ClientImageState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTextureState = TextureState<ImageID, SamplerID>;

impl NamedComponent for ClientTextureState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientMaterialState = MaterialState<TextureID>;
pub type ClientMaterialUpdate = MaterialStateUpdatable<TextureID>;

impl NamedComponent for ClientMaterialState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientTableState = TableState<MethodID, SignalID>;
pub type ClientTableUpdate = TableStateUpdatable<MethodID, SignalID>;

impl NamedComponent for ClientTableState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientPlotUpdate = PlotStateUpdatable<TableID, MethodID, SignalID>;
pub type ClientPlotState = PlotState<TableID, MethodID, SignalID>;

impl NamedComponent for ClientPlotState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientEntityUpdate = EntityStateUpdatable<
    EntityID,
    BufferViewID,
    GeometryID,
    LightID,
    TableID,
    PlotID,
    MethodID,
    SignalID,
>;
pub type ClientEntityState = EntityState<
    EntityID,
    BufferViewID,
    GeometryID,
    LightID,
    TableID,
    PlotID,
    MethodID,
    SignalID,
>;

impl NamedComponent for ClientEntityState {
    fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

pub type ClientDocumentUpdate = DocumentUpdate<MethodID, SignalID>;

pub type ClientMessageSignalInvoke =
    MessageSignalInvoke<EntityID, TableID, PlotID>;

// =============================================================================
pub trait CommComponent {
    fn method_list(&self) -> Option<&[MethodID]>;
    fn signal_list(&self) -> Option<&[SignalID]>;
}

impl CommComponent for ClientEntityState {
    fn method_list(&self) -> Option<&[MethodID]> {
        self.mutable.methods_list.as_ref().map(|f| f.as_slice())
    }

    fn signal_list(&self) -> Option<&[SignalID]> {
        self.mutable.signals_list.as_ref().map(|f| f.as_slice())
    }
}

impl CommComponent for ClientPlotState {
    fn method_list(&self) -> Option<&[MethodID]> {
        self.mutable.methods_list.as_ref().map(|f| f.as_slice())
    }

    fn signal_list(&self) -> Option<&[SignalID]> {
        self.mutable.signals_list.as_ref().map(|f| f.as_slice())
    }
}

impl CommComponent for ClientTableState {
    fn method_list(&self) -> Option<&[MethodID]> {
        self.mutable.methods_list.as_ref().map(|f| f.as_slice())
    }

    fn signal_list(&self) -> Option<&[SignalID]> {
        self.mutable.signals_list.as_ref().map(|f| f.as_slice())
    }
}
