use ciborium::value;
use colabrodo_common::client_communication::InvokeIDType;
use colabrodo_common::nooid::*;
use colabrodo_common::server_communication::DocumentUpdate;
use colabrodo_macros::UpdatableStateItem;
use core::fmt::Debug;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

use colabrodo_common::common;
use colabrodo_common::types::*;

use colabrodo_common::components::*;

pub use colabrodo_common::components::{
    AttributeSemantic, BufferState, BufferViewType, GeometryIndex, LightState,
    MethodArg, MethodState, PrimitiveType, SamplerState, SignalState,
};

use crate::server_state::ComponentCell;
use crate::server_state::MethodResult;
use crate::server_state::ServerStatePtr;

// Traits ==============================================

/// A trait to specify the update mechanism
pub trait UpdatableStateItem {
    type HostState;
    fn patch(self, m: &Self::HostState);
}

pub struct AsyncMethodContent {
    pub state: ServerStatePtr,
    pub context: Option<InvokeIDType>,
    pub args: Vec<value::Value>,
    pub from: uuid::Uuid,
}

pub struct AsyncMethodReply {
    pub result: MethodResult,
}

#[derive(Default)]
pub struct MethodHandlerChannels {
    pub(crate) tx: Option<mpsc::Sender<AsyncMethodContent>>,
    pub(crate) rx: Option<mpsc::Receiver<AsyncMethodReply>>,
}

async fn per_method_spinner<F>(
    f: F,
    mut input: mpsc::Receiver<AsyncMethodContent>,
    output: mpsc::Sender<AsyncMethodReply>,
) where
    F: Fn(AsyncMethodContent) -> MethodResult + Send,
{
    while let Some(m) = input.recv().await {
        let result = f(m);

        if let Err(_) = output.send(AsyncMethodReply { result }).await {
            return;
        }
    }
}

impl MethodHandlerChannels {
    pub fn assign<F>(f: F) -> Self
    where
        F: Fn(AsyncMethodContent) -> MethodResult + Send + 'static,
    {
        let (to_tx, to_rx) = mpsc::channel(1);
        let (from_tx, from_rx) = mpsc::channel(1);

        tokio::spawn(per_method_spinner(f, to_rx, from_tx));

        Self {
            tx: Some(to_tx),
            rx: Some(from_rx),
        }
    }

    pub(crate) async fn activate(
        &mut self,
        content: AsyncMethodContent,
    ) -> Option<AsyncMethodReply> {
        if let Some(channel) = &self.tx {
            if let Ok(_) = channel.send(content).await {
                if let Some(recv_channel) = &mut self.rx {
                    return recv_channel.recv().await;
                }
            }
        }

        None
    }
}

#[derive(Clone, Default)]
pub struct MethodHandlerSlot {
    pub(crate) channels: Option<Arc<Mutex<MethodHandlerChannels>>>,
}

impl MethodHandlerSlot {
    pub fn assign<F>(f: F) -> Self
    where
        F: Fn(AsyncMethodContent) -> MethodResult + Send + 'static,
    {
        Self {
            channels: Some(Arc::new(Mutex::new(
                MethodHandlerChannels::assign(f),
            ))),
        }
    }
}

impl Debug for MethodHandlerSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MethodHandlerSlot").finish()
    }
}

// Component Refs ==============================================

/// A reference to a component.
///
/// This is a lightweight handle to a component; when the last handle goes out of scope, the component will be deleted.
///
/// This handle also hashes based on the underlying content, so it can be used in hash maps, etc.
#[derive(Debug)]
pub struct ComponentReference<IDType, T>(
    pub(crate) Arc<ComponentCell<IDType, T>>,
)
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass;

impl<IDType, T> ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
    pub fn new(ptr: Arc<ComponentCell<IDType, T>>) -> Self {
        Self(ptr)
    }

    pub fn id(&self) -> IDType {
        self.0.id()
    }

    pub(crate) fn send_to_broadcast(&self, rec: Recorder) {
        self.0.send_to_broadcast(rec)
    }
}

impl<IDType, T> Clone for ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<IDType, T> serde::Serialize for ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let id = (*self.0).id();

        id.serialize(serializer)
    }
}

impl<IDType, T> core::hash::Hash for ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state);
    }
}

impl<IDType, T> PartialEq for ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<IDType, T> Eq for ComponentReference<IDType, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
    IDType: IDClass,
{
}

// =============================================================================

/// A simple struct to hold a message and an id for serialization
#[derive(Serialize)]
pub(crate) struct Bouncer<'a, IDType, T> {
    pub id: IDType,

    #[serde(flatten)]
    pub content: &'a T,
}

// Write destination ==============================================

/// In the future this type will probably have a bit more safety on messages and ids.
#[derive(Debug)]
pub struct Recorder {
    pub data: Vec<u8>,
}

impl Recorder {
    pub fn record<T: Serialize>(id: u32, t: &T) -> Self {
        let mut data = Vec::<u8>::new();
        ciborium::ser::into_writer(&(id, &t), &mut data).unwrap();
        Self { data }
    }
}

// Messages ==============================================

pub type ServerMethodState = MethodState<MethodHandlerSlot>;
pub type ServerSignalState = SignalState<()>;

pub type MethodReference = ComponentReference<MethodID, ServerMethodState>;
pub type SignalReference = ComponentReference<SignalID, ServerSignalState>;

// =======================================================

pub type ServerRenderRepresentation =
    RenderRepresentation<GeometryReference, BufferViewReference>;

pub type ServerEntityRepresentation =
    EntityRepresentation<GeometryReference, BufferViewReference>;

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerEntityStateUpdatable {
    pub parent: Option<EntityReference>,

    pub transform: Option<[f32; 16]>,

    #[serde(flatten)]
    pub representation: Option<ServerEntityRepresentation>,

    pub lights: Option<Vec<LightReference>>,
    pub tables: Option<Vec<TableReference>>,
    pub plots: Option<Vec<PlotReference>>,
    pub tags: Option<Vec<String>>,

    pub methods_list: Option<Vec<MethodReference>>,
    pub signals_list: Option<Vec<SignalReference>>,

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

pub type EntityReference = ComponentReference<EntityID, ServerEntityState>;

// ========================================================================

pub type ServerGeometryAttribute = GeometryAttribute<BufferViewReference>;

pub type ServerGeometryIndex = GeometryIndex<BufferViewReference>;

pub type ServerGeometryPatch =
    GeometryPatch<BufferViewReference, MaterialReference>;

pub type ServerGeometryState =
    GeometryState<BufferViewReference, MaterialReference>;

pub type GeometryReference =
    ComponentReference<GeometryID, ServerGeometryState>;

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerTableStateUpdatable {
    pub meta: Option<String>,
    pub methods_list: Option<Vec<MethodReference>>,
    pub signals_list: Option<Vec<SignalReference>>,
}

#[serde_with::skip_serializing_none]
#[derive(Default, Serialize)]
pub struct ServerTableState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: ServerTableStateUpdatable,
}

impl Debug for ServerTableState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerTableState")
            .field("name", &self.name)
            .field("mutable", &self.mutable)
            .finish()
    }
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

pub type TableReference = ComponentReference<TableID, ServerTableState>;

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct ServerPlotStateUpdatable {
    pub table: Option<TableReference>,

    pub methods_list: Option<Vec<MethodReference>>,
    pub signals_list: Option<Vec<SignalReference>>,
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

pub type PlotReference = ComponentReference<PlotID, ServerPlotState>;

// ========================================================================

pub type BufferReference = ComponentReference<BufferID, BufferState>;

pub type ServerBufferViewState = BufferViewState<BufferReference>;

pub type BufferViewReference =
    ComponentReference<BufferViewID, ServerBufferViewState>;

pub trait BufferViewStateHelpers {
    fn new_from_whole_buffer(buffer: BufferReference) -> Self;
}

impl BufferViewStateHelpers for ServerBufferViewState {
    fn new_from_whole_buffer(buffer: BufferReference) -> Self {
        let buffer_size = buffer.0.inspect(|t| t.size).unwrap_or(0);

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

pub type ServerTextureRef = TextureRef<TextureReference>;
pub type ServerPBRInfo = PBRInfo<TextureReference>;

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

pub type MaterialReference =
    ComponentReference<MaterialID, ServerMaterialState>;

// ========================================================================

pub type ServerImageState = ImageState<BufferViewReference>;

pub trait ImageStateHelpers {
    fn new_from_buffer(buffer: BufferViewReference) -> Self;

    fn new_from_url(url: &str) -> Self;
}

impl ImageStateHelpers for ServerImageState {
    fn new_from_buffer(buffer: BufferViewReference) -> Self {
        Self {
            name: None,
            source: ImageSource::new_buffer(buffer),
        }
    }

    fn new_from_url(url: &str) -> Self {
        Self {
            name: None,
            source: ImageSource::new_uri(url.parse().unwrap()),
        }
    }
}

pub type ImageReference = ComponentReference<ImageID, ServerImageState>;

// ========================================================================

pub type SamplerReference = ComponentReference<SamplerID, SamplerState>;

pub type ServerTextureState = TextureState<ImageReference, SamplerReference>;

trait TextureStateHelpers {
    fn new(image: ImageReference, sampler: Option<SamplerReference>) -> Self;
}

impl TextureStateHelpers for ServerTextureState {
    fn new(image: ImageReference, sampler: Option<SamplerReference>) -> Self {
        Self {
            name: Default::default(),
            image,
            sampler,
        }
    }
}

pub type TextureReference = ComponentReference<TextureID, ServerTextureState>;

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

pub type LightReference = ComponentReference<LightID, ServerLightState>;

// =============================================================================

pub type ServerDocumentUpdate =
    DocumentUpdate<MethodReference, SignalReference>;
