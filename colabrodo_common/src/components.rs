use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde_with;

use crate::types::*;

// =============================================================================

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

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct SignalState {
    pub name: String,
    pub doc: Option<String>,
    pub return_doc: Option<String>,
    pub arg_doc: Vec<MethodArg>,
}

// =============================================================================

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

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct BufferViewState<BufferReference> {
    pub name: Option<String>,

    pub source_buffer: BufferReference,

    #[serde(rename = "type")]
    pub view_type: BufferViewType,

    pub offset: u64,
    pub length: u64,
}

// =============================================================================

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
#[derive(Debug, Serialize)]
pub struct GeometryIndex<BufferViewRef> {
    pub view: BufferViewRef,
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
pub struct GeometryPatch<BufferViewRef, MaterialRef> {
    pub attributes: Vec<GeometryAttribute<BufferViewRef>>,
    pub vertex_count: u64,
    pub indices: Option<GeometryIndex<BufferViewRef>>,
    #[serde(rename = "type")]
    pub patch_type: PrimitiveType,
    pub material: MaterialRef,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct GeometryState<BufferViewRef, MaterialRef> {
    pub name: Option<String>,

    pub patches: Vec<GeometryPatch<BufferViewRef, MaterialRef>>,
}

// =============================================================================

#[derive(Debug)]
pub enum ImageSource<BufferViewRef> {
    Buffer(BufferViewRef),
    URI(Url),
}

impl<BufferViewRef> serde::Serialize for ImageSource<BufferViewRef>
where
    BufferViewRef: serde::Serialize,
{
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
pub struct ImageState<BufferViewRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub source: ImageSource<BufferViewRef>,
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct TextureState<ImageStateRef, SamplerStateRef> {
    pub name: Option<String>,

    pub image: ImageStateRef,

    pub sampler: Option<SamplerStateRef>,
}

// =============================================================================

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

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct TextureRef<TextureStateRef> {
    pub texture: TextureStateRef,
    pub transform: Option<Mat3>,
    pub texture_coord_slot: Option<u32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
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
#[derive(Debug, Default, Serialize)]
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
#[derive(Debug, Default, Serialize)]
pub struct MaterialState<TextureStateRef> {
    pub name: Option<String>,

    #[serde(flatten)]
    pub mutable: MaterialStateUpdatable<TextureStateRef>,
}

// =============================================================================

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
pub struct LightStateUpdatable {
    pub color: Option<RGB>,
    pub intensity: Option<f32>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct LightState {
    pub name: Option<String>,

    #[serde(flatten)]
    pub light_type: LightType,

    #[serde(flatten)]
    pub mutable: LightStateUpdatable,
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct PlotStateUpdatable<TableRef, MethodRef, SignalRef> {
    pub table: Option<TableRef>,

    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct PlotState<TableRef, MethodRef, SignalRef> {
    name: Option<String>,

    #[serde(flatten)]
    pub(crate) mutable: PlotStateUpdatable<TableRef, MethodRef, SignalRef>,
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct TableStateUpdatable<MethodRef, SignalRef> {
    pub meta: Option<String>,
    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct TableState<MethodRef, SignalRef> {
    name: Option<String>,

    #[serde(flatten)]
    pub mutable: TableStateUpdatable<MethodRef, SignalRef>,
}

// =============================================================================

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
pub struct RenderRepresentation<GeometryRef> {
    pub mesh: GeometryRef,
    pub instances: Option<InstanceSource>,
}

#[derive(Debug)]
pub enum EntityRepresentation<GeometryRef> {
    Null,
    Text(TextRepresentation),
    Web(WebRepresentation),
    Render(RenderRepresentation<GeometryRef>),
}
impl<GeometryRef> serde::Serialize for EntityRepresentation<GeometryRef>
where
    GeometryRef: serde::Serialize,
{
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
#[derive(Debug, Default, Serialize)]
pub struct EntityStateUpdatable<
    EntityRef,
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
    pub representation: Option<EntityRepresentation<GeometryRef>>,

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
#[derive(Debug, Default, Serialize)]
pub struct EntityState<
    EntityRef,
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
        GeometryRef,
        LightRef,
        TableRef,
        PlotRef,
        MethodRef,
        SignalRef,
    >,
}

// =============================================================================
