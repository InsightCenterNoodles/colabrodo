use ciborium::tag::Required;
use colabrodo_macros::UpdatableStateItem;
use core::fmt::Debug;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::rc::Rc;

use colabrodo_common::nooid::NooID;

use colabrodo_common::common;

use crate::server_state::{ComponentCell, SignalInvokeObj};

// =====================================================

#[derive(Debug, Serialize, Default)]
pub enum Format {
    #[default]
    U8,
    U16,
    U32,

    U8VEC4,

    U16VEC2,

    VEC2,
    VEC3,
    VEC4,

    MAT3,
    MAT4,
}

pub type RGB = [f32; 3];
pub type RGBA = [f32; 4];

pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

pub type Mat3 = [f32; 9];
pub type Mat4 = [f32; 16];

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

// Traits ==============================================

pub trait UpdatableStateItem {
    type HostState;
    fn patch(self, m: &mut Self::HostState);
}

pub trait ServerStateItemMessageIDs {
    fn create_message_id() -> common::ServerMessages;
    fn update_message_id() -> common::ServerMessages;
    fn delete_message_id() -> common::ServerMessages;
}

#[derive(Debug, Serialize)]
pub(crate) struct CommonDeleteMessage {
    pub id: NooID,
}

/// A struct to represent an array of bytes, for proper serialization to CBOR
#[derive(Debug, Default)]
pub struct ByteBuff {
    pub bytes: Vec<u8>,
}

impl ByteBuff {
    pub fn new(data: Vec<u8>) -> Self {
        Self { bytes: data }
    }
}

impl Serialize for ByteBuff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.bytes.as_slice())
    }
}

#[derive(Debug)]
pub struct Url {
    url: Required<String, 32>,
}

impl Url {
    pub fn new(url: String) -> Self {
        Self { url: Required(url) }
    }
    pub fn new_from_slice(url: &str) -> Self {
        Self {
            url: Required(url.to_string()),
        }
    }
}

impl Serialize for Url {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.url.serialize(serializer)
    }
}

// Component Refs ==============================================

#[derive(Debug)]
pub struct ComponentReference<T>(pub(crate) Rc<ComponentCell<T>>)
where
    T: Serialize + ServerStateItemMessageIDs + Debug;

impl<T> ComponentReference<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
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
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> serde::Serialize for ComponentReference<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
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
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state);
    }
}

impl<T> PartialEq for ComponentReference<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for ComponentReference<T> where
    T: Serialize + ServerStateItemMessageIDs + Debug
{
}

// =============================================================================

#[derive(Serialize)]
pub struct Bouncer<'a, T> {
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

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
pub struct MethodArg {
    pub name: String,
    pub doc: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MethodState {
    pub name: String,
    pub doc: Option<String>,
    pub return_doc: Option<String>,
    pub arg_doc: Vec<MethodArg>,
}

impl ServerStateItemMessageIDs for MethodState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgMethodCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgMethodDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct SignalState {
    pub name: String,
    pub doc: Option<String>,
    pub return_doc: Option<String>,
    pub arg_doc: Vec<MethodArg>,
}

impl ServerStateItemMessageIDs for SignalState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgSignalCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgSignalDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct TextRepresentation {
    pub txt: String,
    pub font: Option<String>,
    pub height: Option<f32>,
    pub width: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct WebRepresentation {
    pub source: Url,
    pub height: Option<f32>,
    pub width: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct InstanceSource {}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct RenderRepresentation {
    pub mesh: ComponentReference<GeometryState>,
    pub instances: Option<InstanceSource>,
}

#[derive(Debug)]
pub enum EntityRepresentation {
    Null,
    Text(TextRepresentation),
    Web(WebRepresentation),
    Render(RenderRepresentation),
}

impl serde::Serialize for EntityRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Representation", 1)?;
        match self {
            EntityRepresentation::Null => {
                s.serialize_field("null_rep", "null")?
            }
            EntityRepresentation::Text(t) => {
                s.serialize_field("text_rep", t)?
            }
            EntityRepresentation::Web(w) => s.serialize_field("web_rep", w)?,
            EntityRepresentation::Render(r) => {
                s.serialize_field("render_rep", r)?
            }
        }
        s.end()
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct EntityStateUpdatable {
    pub parent: Option<ComponentReference<EntityState>>,

    pub transform: Option<[f32; 16]>,

    #[serde(flatten)]
    pub representation: Option<EntityRepresentation>,

    pub lights: Option<Vec<ComponentReference<LightState>>>,
    pub tables: Option<Vec<ComponentReference<TableState>>>,
    pub plots: Option<Vec<ComponentReference<PlotState>>>,
    pub tags: Option<Vec<String>>,

    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,

    pub influence: Option<BoundingBox>,
    pub visible: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct EntityState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub extra: EntityStateUpdatable,
}

impl ServerStateItemMessageIDs for EntityState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgEntityUpdate
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgEntityCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgEntityDelete
    }
}

// ========================================================================

#[derive(Debug, Default, Serialize)]
pub enum AttributeSemantic {
    #[default]
    #[serde(rename = "POSITION")]
    Position,
    #[serde(rename = "NORMAL")]
    Normal,
    #[serde(rename = "TANGENT")]
    Tangent,
    #[serde(rename = "TEXTURE")]
    Texture,
    #[serde(rename = "COLOR")]
    Color,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct GeometryAttribute {
    pub view: ComponentReference<BufferViewState>,
    pub semantic: AttributeSemantic,
    pub channel: Option<u32>,
    pub offset: Option<u32>,
    pub stride: Option<u32>,
    pub format: Format,
    pub minimum_value: Option<Vec<f32>>,
    pub maximum_value: Option<Vec<f32>>,
    pub normalized: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct GeometryIndex {
    pub view: ComponentReference<BufferViewState>,
    pub count: u32,
    pub offset: Option<u32>,
    pub stride: Option<u32>,
    pub format: Format,
}

#[derive(Debug, Default, Serialize)]
pub enum PrimitiveType {
    #[default]
    #[serde(rename = "POINTS")]
    Points,
    #[serde(rename = "LINES")]
    Lines,
    #[serde(rename = "LINE_LOOP")]
    LineLoop,
    #[serde(rename = "LINE_STRIP")]
    LineStrip,
    #[serde(rename = "TRIANGLES")]
    Triangles,
    #[serde(rename = "TRIANGLE_STRIP")]
    TriangleStrip,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct GeometryPatch {
    pub attributes: Vec<GeometryAttribute>,
    pub vertex_count: u64,
    pub indices: Option<GeometryIndex>,
    #[serde(rename = "type")]
    pub patch_type: PrimitiveType,
    pub material: ComponentReference<MaterialState>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct GeometryState {
    pub name: Option<String>,

    pub patches: Vec<GeometryPatch>,
}

impl ServerStateItemMessageIDs for GeometryState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgGeometryCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgGeometryDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct TableStateUpdatable {
    id: NooID,

    pub meta: Option<String>,
    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct TableState {
    name: Option<String>,

    #[serde(flatten)]
    pub extra: TableStateUpdatable,
}

impl ServerStateItemMessageIDs for TableState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgTableUpdate
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgTableCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgTableDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct PlotStateUpdatable {
    id: NooID,

    pub table: Option<ComponentReference<TableState>>,

    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct PlotState {
    name: Option<String>,

    #[serde(flatten)]
    pub(crate) extra: PlotStateUpdatable,
}

impl ServerStateItemMessageIDs for PlotState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgPlotUpdate
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgPlotCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgPlotDelete
    }
}

// ========================================================================

#[derive(Debug)]
pub enum BufferRepresentation {
    Inline(ByteBuff),
    URI(Url),
}

impl Default for BufferRepresentation {
    fn default() -> Self {
        Self::Inline(ByteBuff::default())
    }
}

impl serde::Serialize for BufferRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Representation", 1)?;
        match self {
            BufferRepresentation::Inline(i) => {
                s.serialize_field("inline_bytes", i)?
            }
            BufferRepresentation::URI(t) => {
                s.serialize_field("uri_bytes", t)?
            }
        }
        s.end()
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct BufferState {
    pub name: Option<String>,

    pub size: u64,

    #[serde(flatten)]
    pub representation: BufferRepresentation,
}

impl BufferState {
    pub fn new_from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            name: None,
            size: bytes.len() as u64,
            representation: BufferRepresentation::Inline(ByteBuff::new(bytes)),
        }
    }

    pub fn new_from_url(url: &str, buffer_size: u64) -> Self {
        Self {
            name: None,
            size: buffer_size,
            representation: BufferRepresentation::URI(Url::new_from_slice(url)),
        }
    }
}

impl ServerStateItemMessageIDs for BufferState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgBufferCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgBufferDelete
    }
}

// ========================================================================

#[derive(Debug, Serialize, Default)]
pub enum BufferViewType {
    #[default]
    #[serde(rename = "UNK")]
    Unknown,
    #[serde(rename = "GEOMETRY")]
    Geometry,
    #[serde(rename = "IMAGE")]
    Image,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct BufferViewState {
    pub name: Option<String>,

    pub source_buffer: ComponentReference<BufferState>,

    #[serde(rename = "type")]
    pub view_type: BufferViewType,

    pub offset: u64,
    pub length: u64,
}

impl BufferViewState {
    pub fn new_from_whole_buffer(
        buffer: ComponentReference<BufferState>,
    ) -> Self {
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

impl ServerStateItemMessageIDs for BufferViewState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgBufferViewCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgBufferViewDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct TextureRef {
    pub texture: ComponentReference<TextureState>,
    pub transform: Option<Mat3>,
    pub texture_coord_slot: Option<u32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct PBRInfo {
    pub base_color: RGBA,
    pub base_color_texture: Option<TextureRef>,
    pub metallic: Option<f32>,
    pub roughness: Option<f32>,
    pub metal_rough_texture: Option<TextureRef>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct MaterialStateUpdatable {
    pub pbr_info: Option<PBRInfo>,
    pub normal_texture: Option<TextureRef>,

    pub occlusion_texture: Option<TextureRef>,
    pub occlusion_texture_factor: Option<f32>,

    pub emissive_texture: Option<TextureRef>,
    pub emissive_factor: Option<Vec3>,

    pub use_alpha: Option<bool>,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MaterialState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub extra: MaterialStateUpdatable,
}

impl ServerStateItemMessageIDs for MaterialState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgMaterialUpdate
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgMaterialCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgMaterialDelete
    }
}

// ========================================================================

#[derive(Debug)]
pub enum ImageSource {
    Buffer(ComponentReference<BufferViewState>),
    URI(Url),
}

impl serde::Serialize for ImageSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ImageSource", 1)?;
        match self {
            ImageSource::Buffer(buffer) => {
                s.serialize_field("buffer_source", buffer)?
            }
            ImageSource::URI(uri) => s.serialize_field("uri_source", uri)?,
        }
        s.end()
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct ImageState {
    pub name: Option<String>,

    #[serde(flatten)]
    source: ImageSource,
}

impl ImageState {
    pub fn new_from_buffer(
        buffer: ComponentReference<BufferViewState>,
    ) -> Self {
        Self {
            name: None,
            source: ImageSource::Buffer(buffer),
        }
    }

    pub fn new_from_url(url: &str) -> Self {
        Self {
            name: None,
            source: ImageSource::URI(Url::new_from_slice(url)),
        }
    }
}

impl ServerStateItemMessageIDs for ImageState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgImageCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgImageDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct TextureState {
    pub name: Option<String>,

    pub image: ComponentReference<ImageState>,

    pub sampler: Option<ComponentReference<SamplerState>>,
}

impl TextureState {
    pub fn new(
        image: ComponentReference<ImageState>,
        sampler: Option<ComponentReference<SamplerState>>,
    ) -> Self {
        Self {
            name: Default::default(),
            image,
            sampler,
        }
    }
}

impl ServerStateItemMessageIDs for TextureState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgTextureCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgTextureDelete
    }
}

// ========================================================================

#[derive(Debug, Serialize)]
pub enum MagFilter {
    #[serde(rename = "NEAREST")]
    Nearest,
    #[serde(rename = "LINEAR")]
    Linear,
}

#[derive(Debug, Serialize)]
pub enum MinFilter {
    #[serde(rename = "NEAREST")]
    Nearest,
    #[serde(rename = "LINEAR")]
    Linear,
    #[serde(rename = "LINEAR_MIPMAP_LINEAR")]
    LinearMipmapLinear,
}

#[derive(Debug, Serialize)]
pub enum SamplerMode {
    #[serde(rename = "CLAMP_TO_EDGE")]
    Clamp,
    #[serde(rename = "MIRRORED_REPEAT")]
    MirrorRepeat,
    #[serde(rename = "REPEAT")]
    Repeat,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
pub struct SamplerState {
    pub name: Option<String>,

    pub mag_filter: Option<MagFilter>,
    pub min_filter: Option<MinFilter>,

    pub wrap_s: Option<SamplerMode>,
    pub wrap_t: Option<SamplerMode>,
}

impl ServerStateItemMessageIDs for SamplerState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::Unknown
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgSamplerCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgSamplerDelete
    }
}

// ========================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, UpdatableStateItem)]
pub struct LightStateUpdatable {
    id: NooID,

    color: Option<RGB>,
    intensity: Option<f32>,
}

#[derive(Debug, Default, Serialize)]
pub struct PointLight {
    range: f32,
}

#[derive(Debug, Default, Serialize)]
pub struct SpotLight {
    range: f32,
    inner_cone_angle_rad: f32,
    outer_cone_angle_rad: f32,
}

#[derive(Debug, Default, Serialize)]
pub struct DirectionalLight {
    range: f32,
}

#[derive(Debug)]
pub enum LightType {
    Point(PointLight),
    Spot(SpotLight),
    Sun(DirectionalLight),
}

impl Default for LightType {
    fn default() -> Self {
        LightType::Point(PointLight::default())
    }
}

impl serde::Serialize for LightType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("LightType", 1)?;
        match self {
            LightType::Point(point) => s.serialize_field("point", point)?,
            LightType::Spot(spot) => s.serialize_field("spot", spot)?,
            LightType::Sun(sun) => s.serialize_field("directional", sun)?,
        }
        s.end()
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct LightState {
    name: Option<String>,

    #[serde(flatten)]
    light_type: LightType,

    #[serde(flatten)]
    extra: LightStateUpdatable,
}

impl ServerStateItemMessageIDs for LightState {
    fn update_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgLightUpdate
    }

    fn create_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgLightCreate
    }

    fn delete_message_id() -> common::ServerMessages {
        common::ServerMessages::MsgLightDelete
    }
}

// ========================================================================
// One off messages

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct DocumentUpdate {
    pub methods_list: Option<Vec<ComponentReference<MethodState>>>,
    pub signals_list: Option<Vec<ComponentReference<SignalState>>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize)]
pub struct MessageSignalInvoke {
    pub id: NooID,
    pub context: Option<SignalInvokeObj>,
    pub signal_data: Vec<ciborium::value::Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MethodException {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<ciborium::value::Value>,
}

impl MethodException {
    pub fn method_not_found(extra: Option<String>) -> Self {
        Self {
            code: ExceptionCodes::MethodNotFound as i32,
            message: extra,
            ..Default::default()
        }
    }
}

pub enum ExceptionCodes {
    MethodNotFound = -32601,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MessageMethodReply {
    pub invoke_id: String,
    pub result: Option<ciborium::value::Value>,
    pub method_exception: Option<MethodException>,
}
