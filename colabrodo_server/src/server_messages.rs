use colabrodo_common::server_communication::DocumentUpdate;
use colabrodo_macros::UpdatableStateItem;
use core::fmt::Debug;
use serde::Serialize;
use std::rc::Rc;

use colabrodo_common::common;
use colabrodo_common::nooid::NooID;
use colabrodo_common::types::*;

use colabrodo_common::components::*;

pub use colabrodo_common::components::{
    AttributeSemantic, BufferRepresentation, BufferState, BufferViewType,
    GeometryIndex, LightState, MethodArg, MethodState, PrimitiveType,
    SamplerState, SignalState,
};

use crate::server_state::ComponentCell;

// Traits ==============================================

pub trait UpdatableStateItem {
    type HostState;
    fn patch(self, m: &mut Self::HostState);
}

// Component Refs ==============================================

#[derive(Debug)]
pub struct ComponentReference<T>(pub(crate) Rc<ComponentCell<T>>)
where
    T: Serialize + ComponentMessageIDs + Debug;

impl<T> ComponentReference<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    pub fn new(ptr: Rc<ComponentCell<T>>) -> Self {
        Self(ptr)
    }

    pub fn id(&self) -> NooID {
        self.0.id()
    }

    pub(crate) fn send_to_broadcast(&self, rec: Recorder) {
        self.0.send_to_broadcast(rec)
    }
}

impl<T> Clone for ComponentReference<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> serde::Serialize for ComponentReference<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let id = (*self.0).id();

        id.serialize(serializer)
    }
}

impl<T> core::hash::Hash for ComponentReference<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state);
    }
}

impl<T> PartialEq for ComponentReference<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for ComponentReference<T> where
    T: Serialize + ComponentMessageIDs + Debug
{
}

// =============================================================================

#[derive(Serialize)]
pub(crate) struct Bouncer<'a, T> {
    pub id: NooID,

    #[serde(flatten)]
    pub content: &'a T,
}

// Write destination ==============================================

#[derive(Debug, Default)]
pub struct Recorder {
    pub data: Vec<u8>,
}

// Messages ==============================================

pub type ServerRenderRepresentation =
    RenderRepresentation<ComponentReference<ServerGeometryState>>;

pub type ServerEntityRepresentation =
    EntityRepresentation<ComponentReference<ServerGeometryState>>;

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerEntityStateUpdatable {
    pub parent: Option<ComponentReference<ServerEntityState>>,

    pub transform: Option<[f32; 16]>,

    #[serde(flatten)]
    pub representation: Option<ServerEntityRepresentation>,

    pub lights: Option<Vec<ComponentReference<ServerLightState>>>,
    pub tables: Option<Vec<ComponentReference<ServerTableState>>>,
    pub plots: Option<Vec<ComponentReference<ServerPlotState>>>,
    pub tags: Option<Vec<String>>,

    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,

    pub influence: Option<BoundingBox>,
    pub visible: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct ServerEntityState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: ServerEntityStateUpdatable,
}

impl ComponentMessageIDs for ServerEntityState {
    fn update_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgEntityUpdate
    }

    fn create_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgEntityCreate
    }

    fn delete_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgEntityDelete
    }
}

// ========================================================================

pub type ServerGeometryAttribute =
    GeometryAttribute<ComponentReference<ServerBufferViewState>>;

pub type ServerGeometryIndex =
    GeometryIndex<ComponentReference<ServerBufferViewState>>;

pub type ServerGeometryPatch = GeometryPatch<
    ComponentReference<ServerBufferViewState>,
    ComponentReference<ServerMaterialState>,
>;

pub type ServerGeometryState = GeometryState<
    ComponentReference<ServerBufferViewState>,
    ComponentReference<ServerMaterialState>,
>;

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerTableStateUpdatable {
    pub meta: Option<String>,
    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct ServerTableState {
    name: Option<String>,

    #[serde(flatten)]
    pub mutable: ServerTableStateUpdatable,
}

impl ComponentMessageIDs for ServerTableState {
    fn update_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgTableUpdate
    }

    fn create_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgTableCreate
    }

    fn delete_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgTableDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerPlotStateUpdatable {
    pub table: Option<ComponentReference<ServerTableState>>,

    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct ServerPlotState {
    name: Option<String>,

    #[serde(flatten)]
    pub(crate) mutable: ServerPlotStateUpdatable,
}

impl ComponentMessageIDs for ServerPlotState {
    fn update_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgPlotUpdate
    }

    fn create_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgPlotCreate
    }

    fn delete_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgPlotDelete
    }
}

// ========================================================================

// ========================================================================

pub type ServerBufferViewState =
    BufferViewState<ComponentReference<BufferState>>;

pub trait BufferViewStateHelpers {
    fn new_from_whole_buffer(buffer: ComponentReference<BufferState>) -> Self;
}

impl BufferViewStateHelpers for ServerBufferViewState {
    fn new_from_whole_buffer(buffer: ComponentReference<BufferState>) -> Self {
        let buffer_size = buffer.0.get().size;

        Self {
            name: None,
            source_buffer: buffer,
            view_type: BufferViewType::Unknown,
            offset: 0,
            length: buffer_size,
        }
    }
}

// ========================================================================

pub type ServerTextureRef = TextureRef<ComponentReference<ServerTextureState>>;
pub type ServerPBRInfo = PBRInfo<ComponentReference<ServerTextureState>>;

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerMaterialStateUpdatable {
    pub pbr_info: Option<ServerPBRInfo>,
    pub normal_texture: Option<ServerTextureRef>,

    pub occlusion_texture: Option<ServerTextureRef>,
    pub occlusion_texture_factor: Option<f32>,

    pub emissive_texture: Option<ServerTextureRef>,
    pub emissive_factor: Option<Vec3>,

    pub use_alpha: Option<bool>,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct ServerMaterialState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: ServerMaterialStateUpdatable,
}

impl ComponentMessageIDs for ServerMaterialState {
    fn update_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgMaterialUpdate
    }

    fn create_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgMaterialCreate
    }

    fn delete_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgMaterialDelete
    }
}

// ========================================================================

pub type ServerImageState =
    ImageState<ComponentReference<ServerBufferViewState>>;

pub trait ImageStateHelpers {
    fn new_from_buffer(
        buffer: ComponentReference<ServerBufferViewState>,
    ) -> Self;

    fn new_from_url(url: &str) -> Self;
}

impl ImageStateHelpers for ServerImageState {
    fn new_from_buffer(
        buffer: ComponentReference<ServerBufferViewState>,
    ) -> Self {
        Self {
            name: None,
            source: ImageSource::new_buffer(buffer),
        }
    }

    fn new_from_url(url: &str) -> Self {
        Self {
            name: None,
            source: ImageSource::new_uri(Url::new_from_slice(url)),
        }
    }
}

// ========================================================================

pub type ServerTextureState = TextureState<
    ComponentReference<ServerImageState>,
    ComponentReference<SamplerState>,
>;

trait TextureStateHelpers {
    fn new(
        image: ComponentReference<ServerImageState>,
        sampler: Option<ComponentReference<SamplerState>>,
    ) -> Self;
}

impl TextureStateHelpers for ServerTextureState {
    fn new(
        image: ComponentReference<ServerImageState>,
        sampler: Option<ComponentReference<SamplerState>>,
    ) -> Self {
        Self {
            name: Default::default(),
            image,
            sampler,
        }
    }
}

// ========================================================================

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerLightStateUpdatable {
    pub color: Option<RGB>,
    pub intensity: Option<f32>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct ServerLightState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub light_type: LightType,

    #[serde(flatten)]
    pub mutable: LightStateUpdatable,
}

impl ComponentMessageIDs for ServerLightState {
    fn update_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgLightUpdate
    }

    fn create_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgLightCreate
    }

    fn delete_message_id() -> common::ServerMessageIDs {
        common::ServerMessageIDs::MsgLightDelete
    }
}

// =============================================================================

pub type ServerDocumentUpdate = DocumentUpdate<
    ComponentReference<MethodState>,
    ComponentReference<SignalState>,
>;
