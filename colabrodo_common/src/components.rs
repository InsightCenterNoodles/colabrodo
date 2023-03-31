//! Component related messages
//!
//! These are templated based on how the component refers to others.

use colabrodo_macros::DeltaPatch;
use serde::{Deserialize, Serialize};
use serde_with::{self, serde_as, DefaultOnError};

use crate::{common::ServerMessageIDs, types::*};

pub trait ComponentMessageIDs {
    fn create_message_id() -> ServerMessageIDs;
    fn update_message_id() -> ServerMessageIDs;
    fn delete_message_id() -> ServerMessageIDs;
}

pub trait DeltaPatch {
    fn patch(&mut self, other: Self);
}

pub trait UpdatableWith {
    type Substate;
    fn update(&mut self, s: Self::Substate);
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MethodArg {
    pub name: String,
    pub doc: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MethodState<Extra> {
    pub name: String,
    pub doc: Option<String>,
    pub return_doc: Option<String>,
    pub arg_doc: Vec<MethodArg>,

    #[serde(skip)]
    pub state: Extra,
}

impl<Extra> ComponentMessageIDs for MethodState<Extra> {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgMethodCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgMethodDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SignalState<Extra> {
    pub name: String,
    pub doc: Option<String>,
    pub arg_doc: Vec<MethodArg>,

    #[serde(skip)]
    pub state: Extra,
}

impl<Extra> ComponentMessageIDs for SignalState<Extra> {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgSignalCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgSignalDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BufferState {
    pub name: Option<String>,

    pub size: u64,

    pub inline_bytes: Option<ByteBuff>,
    pub uri_bytes: Option<url::Url>,
}

impl BufferState {
    pub fn new_from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            name: None,
            size: bytes.len() as u64,
            inline_bytes: Some(ByteBuff::new(bytes)),
            uri_bytes: None,
        }
    }

    pub fn new_from_url(url: &str, buffer_size: u64) -> Self {
        Self {
            name: None,
            size: buffer_size,
            inline_bytes: None,
            uri_bytes: Some(url.parse().unwrap()),
        }
    }
}

impl ComponentMessageIDs for BufferState {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgBufferCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgBufferDelete
    }
}

// =============================================================================

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum BufferViewType {
    #[default]
    #[serde(rename = "UNK")]
    Unknown,
    #[serde(rename = "GEOMETRY")]
    Geometry,
    #[serde(rename = "IMAGE")]
    Image,
}

#[serde_as]
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct BufferViewState<BufferReference> {
    pub name: Option<String>,

    pub source_buffer: BufferReference,

    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(rename = "type")]
    pub view_type: BufferViewType,

    pub offset: u64,
    pub length: u64,
}

impl<BufferReference> ComponentMessageIDs for BufferViewState<BufferReference> {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgBufferViewCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgBufferViewDelete
    }
}

// =============================================================================

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct GeometryAttribute<BufferViewRef> {
    pub view: BufferViewRef,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct GeometryIndex<BufferViewRef> {
    pub view: BufferViewRef,
    pub count: u32,
    pub offset: Option<u32>,
    pub stride: Option<u32>,
    pub format: Format,
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct GeometryPatch<BufferViewRef, MaterialRef> {
    pub attributes: Vec<GeometryAttribute<BufferViewRef>>,
    pub vertex_count: u64,
    pub indices: Option<GeometryIndex<BufferViewRef>>,
    #[serde(rename = "type")]
    pub patch_type: PrimitiveType,
    pub material: MaterialRef,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GeometryState<BufferViewRef, MaterialRef> {
    pub name: Option<String>,

    pub patches: Vec<GeometryPatch<BufferViewRef, MaterialRef>>,
}

impl<BufferViewRef, MaterialRef> ComponentMessageIDs
    for GeometryState<BufferViewRef, MaterialRef>
{
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgGeometryCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgGeometryDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageSource<BufferViewRef> {
    buffer_source: Option<BufferViewRef>,
    uri_source: Option<url::Url>,
}

impl<BufferViewRef> ImageSource<BufferViewRef> {
    pub fn new_buffer(buffer: BufferViewRef) -> Self {
        Self {
            buffer_source: Some(buffer),
            uri_source: None,
        }
    }

    pub fn new_uri(uri: url::Url) -> Self {
        Self {
            buffer_source: None,
            uri_source: Some(uri),
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageState<BufferViewRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub source: ImageSource<BufferViewRef>,
}

impl<BufferViewRef> ComponentMessageIDs for ImageState<BufferViewRef> {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgImageCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgImageDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct TextureState<ImageStateRef, SamplerStateRef> {
    pub name: Option<String>,

    pub image: ImageStateRef,

    pub sampler: Option<SamplerStateRef>,
}

impl<ImageStateRef, SamplerStateRef> ComponentMessageIDs
    for TextureState<ImageStateRef, SamplerStateRef>
{
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgTextureCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgTextureDelete
    }
}

// =============================================================================

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MagFilter {
    #[serde(rename = "NEAREST")]
    Nearest,
    #[serde(rename = "LINEAR")]
    Linear,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MinFilter {
    #[serde(rename = "NEAREST")]
    Nearest,
    #[serde(rename = "LINEAR")]
    Linear,
    #[serde(rename = "LINEAR_MIPMAP_LINEAR")]
    LinearMipmapLinear,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SamplerMode {
    #[serde(rename = "CLAMP_TO_EDGE")]
    Clamp,
    #[serde(rename = "MIRRORED_REPEAT")]
    MirrorRepeat,
    #[serde(rename = "REPEAT")]
    Repeat,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default, Deserialize)]
pub struct SamplerState {
    pub name: Option<String>,

    pub mag_filter: Option<MagFilter>,
    pub min_filter: Option<MinFilter>,

    pub wrap_s: Option<SamplerMode>,
    pub wrap_t: Option<SamplerMode>,
}

impl ComponentMessageIDs for SamplerState {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::Unknown
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgSamplerCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgSamplerDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct TextureRef<TextureStateRef> {
    pub texture: TextureStateRef,
    pub transform: Option<Mat3>,
    pub texture_coord_slot: Option<u32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct PBRInfo<TextureStateRef> {
    pub base_color: RGBA,
    pub base_color_texture: Option<TextureRef<TextureStateRef>>,
    pub metallic: Option<f32>,
    pub roughness: Option<f32>,
    pub metal_rough_texture: Option<TextureRef<TextureStateRef>>,
}

impl<TextureStateRef> Default for PBRInfo<TextureStateRef> {
    fn default() -> Self {
        Self {
            base_color: Default::default(),
            base_color_texture: None,
            metallic: Default::default(),
            roughness: Default::default(),
            metal_rough_texture: None,
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, DeltaPatch)]
#[patch_generic(TextureStateRef)]
pub struct MaterialStateUpdatable<TextureStateRef> {
    pub pbr_info: Option<PBRInfo<TextureStateRef>>,
    pub normal_texture: Option<TextureRef<TextureStateRef>>,

    pub occlusion_texture: Option<TextureRef<TextureStateRef>>,
    pub occlusion_texture_factor: Option<f32>,

    pub emissive_texture: Option<TextureRef<TextureStateRef>>,
    pub emissive_factor: Option<Vec3>,

    pub use_alpha: Option<bool>,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaterialState<TextureStateRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: MaterialStateUpdatable<TextureStateRef>,
}

impl<TextureStateRef> UpdatableWith for MaterialState<TextureStateRef> {
    type Substate = MaterialStateUpdatable<TextureStateRef>;

    fn update(&mut self, s: Self::Substate) {
        self.mutable.patch(s);
    }
}

impl<TextureStateRef> ComponentMessageIDs for MaterialState<TextureStateRef> {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgMaterialUpdate
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgMaterialCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgMaterialDelete
    }
}

// =============================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PointLight {
    range: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SpotLight {
    range: f32,
    inner_cone_angle_rad: f32,
    outer_cone_angle_rad: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DirectionalLight {
    range: f32,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct LightType {
    point: Option<PointLight>,
    spot: Option<SpotLight>,
    directional: Option<DirectionalLight>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, DeltaPatch)]
pub struct LightStateUpdatable {
    pub color: Option<RGB>,
    pub intensity: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct LightState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub light_type: LightType,

    #[serde(flatten)]
    pub mutable: LightStateUpdatable,
}

impl UpdatableWith for LightState {
    type Substate = LightStateUpdatable;

    fn update(&mut self, s: Self::Substate) {
        self.mutable.patch(s);
    }
}

impl ComponentMessageIDs for LightState {
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgLightUpdate
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgLightCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgLightDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, DeltaPatch)]
#[patch_generic(TableRef, MethodRef, SignalRef)]
pub struct PlotStateUpdatable<TableRef, MethodRef, SignalRef> {
    pub table: Option<TableRef>,

    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PlotState<TableRef, MethodRef, SignalRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: PlotStateUpdatable<TableRef, MethodRef, SignalRef>,
}

impl<TableRef, MethodRef, SignalRef> UpdatableWith
    for PlotState<TableRef, MethodRef, SignalRef>
{
    type Substate = PlotStateUpdatable<TableRef, MethodRef, SignalRef>;
    fn update(&mut self, s: Self::Substate) {
        self.mutable.patch(s);
    }
}

impl<TableRef, MethodRef, SignalRef> ComponentMessageIDs
    for PlotState<TableRef, MethodRef, SignalRef>
{
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgPlotUpdate
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgPlotCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgPlotDelete
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, DeltaPatch)]
#[patch_generic(MethodRef, SignalRef)]
pub struct TableStateUpdatable<MethodRef, SignalRef> {
    pub meta: Option<String>,
    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableState<MethodRef, SignalRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: TableStateUpdatable<MethodRef, SignalRef>,
}

impl<MethodRef, SignalRef> UpdatableWith for TableState<MethodRef, SignalRef> {
    type Substate = TableStateUpdatable<MethodRef, SignalRef>;
    fn update(&mut self, s: Self::Substate) {
        self.mutable.patch(s);
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub txt: String,
    pub font: Option<String>,
    pub height: Option<f32>,
    pub width: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct WebRepresentation {
    pub source: url::Url,
    pub height: Option<f32>,
    pub width: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InstanceSource<BufferViewRef> {
    pub view: BufferViewRef,
    pub stride: Option<u64>,
    pub bb: Option<BoundingBox>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct RenderRepresentation<GeometryRef, BufferViewRef> {
    pub mesh: GeometryRef,
    pub instances: Option<InstanceSource<BufferViewRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct NullRepresentation;

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EntityRepresentation<GeometryRef, BufferViewRef> {
    null_rep: Option<NullRepresentation>,
    text_rep: Option<TextRepresentation>,
    web_rep: Option<WebRepresentation>,
    render_rep: Option<RenderRepresentation<GeometryRef, BufferViewRef>>,
}

impl<GeometryRef, BufferViewRef>
    EntityRepresentation<GeometryRef, BufferViewRef>
{
    pub fn new_null() -> Self {
        Self {
            null_rep: Some(NullRepresentation),
            text_rep: None,
            web_rep: None,
            render_rep: None,
        }
    }

    pub fn new_text(t: TextRepresentation) -> Self {
        Self {
            null_rep: None,
            text_rep: Some(t),
            web_rep: None,
            render_rep: None,
        }
    }

    pub fn new_web(t: WebRepresentation) -> Self {
        Self {
            null_rep: None,
            text_rep: None,
            web_rep: Some(t),
            render_rep: None,
        }
    }

    pub fn new_render(
        t: RenderRepresentation<GeometryRef, BufferViewRef>,
    ) -> Self {
        Self {
            null_rep: None,
            text_rep: None,
            web_rep: None,
            render_rep: Some(t),
        }
    }
}

// This class is just terrible...

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, DeltaPatch)]
#[patch_generic(
    EntityRef,
    BufferViewRef,
    GeometryRef,
    LightRef,
    TableRef,
    PlotRef,
    MethodRef,
    SignalRef
)]
pub struct EntityStateUpdatable<
    EntityRef,
    BufferViewRef,
    GeometryRef,
    LightRef,
    TableRef,
    PlotRef,
    MethodRef,
    SignalRef,
> {
    pub parent: Option<EntityRef>,

    pub transform: Option<[f32; 16]>,

    #[serde(flatten)]
    pub representation:
        Option<EntityRepresentation<GeometryRef, BufferViewRef>>,

    pub lights: Option<Vec<LightRef>>,
    pub tables: Option<Vec<TableRef>>,
    pub plots: Option<Vec<PlotRef>>,
    pub tags: Option<Vec<String>>,

    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,

    pub influence: Option<BoundingBox>,
    pub visible: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EntityState<
    EntityRef,
    BufferViewRef,
    GeometryRef,
    LightRef,
    TableRef,
    PlotRef,
    MethodRef,
    SignalRef,
> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: EntityStateUpdatable<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    >,
}

impl<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    > UpdatableWith
    for EntityState<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    >
{
    type Substate = EntityStateUpdatable<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    >;
    fn update(&mut self, s: Self::Substate) {
        self.mutable.patch(s);
    }
}

impl<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    > ComponentMessageIDs
    for EntityState<
        EntityRef,
        BufferViewRef,
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    >
{
    fn update_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgEntityUpdate
    }

    fn create_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgEntityCreate
    }

    fn delete_message_id() -> ServerMessageIDs {
        ServerMessageIDs::MsgEntityDelete
    }
}

// =============================================================================
// =============================================================================
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::components::BufferState;

    #[test]
    fn buffer_state_serde() {
        let rep = BufferState::new_from_bytes(vec![10, 11, 12]);

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&rep, &mut pack).expect("Pack");

        let other: BufferState =
            ciborium::de::from_reader(pack.as_slice()).unwrap();

        assert_eq!(rep, other);
    }

    #[test]
    fn buffer_state_serde_url() {
        let rep = BufferState::new_from_url("http://wombat.com", 1024);

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&rep, &mut pack).expect("Pack");

        let other: BufferState =
            ciborium::de::from_reader(pack.as_slice()).unwrap();

        assert_eq!(rep, other);
    }
}
