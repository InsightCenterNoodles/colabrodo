use thiserror::Error;

use ciborium::value;
use num_derive::FromPrimitive;

use crate::nooid::NooID;

/// The type of noodles component being operated on
#[derive(Debug, Default, Clone, Copy)]
pub enum ComponentType {
    Entity,
    Table,
    Plot,

    Geometry,
    Material,

    Light,

    Texture,
    Image,
    Sampler,

    BufferView,
    Buffer,

    Method,
    Signal,

    Document,

    #[default]
    None,
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Discriminant for server message types
#[derive(FromPrimitive, Clone, Copy, Debug)]
pub enum ServerMessages {
    MsgMethodCreate = 0,
    MsgMethodDelete = 1,
    MsgSignalCreate = 2,
    MsgSignalDelete = 3,
    MsgEntityCreate = 4,
    MsgEntityUpdate = 5,
    MsgEntityDelete = 6,
    MsgPlotCreate = 7,
    MsgPlotUpdate = 8,
    MsgPlotDelete = 9,
    MsgBufferCreate = 10,
    MsgBufferDelete = 11,
    MsgBufferViewCreate = 12,
    MsgBufferViewDelete = 13,
    MsgMaterialCreate = 14,
    MsgMaterialUpdate = 15,
    MsgMaterialDelete = 16,
    MsgImageCreate = 17,
    MsgImageDelete = 18,
    MsgTextureCreate = 19,
    MsgTextureDelete = 20,
    MsgSamplerCreate = 21,
    MsgSamplerDelete = 22,
    MsgLightCreate = 23,
    MsgLightUpdate = 24,
    MsgLightDelete = 25,
    MsgGeometryCreate = 26,
    MsgGeometryDelete = 27,
    MsgTableCreate = 28,
    MsgTableUpdate = 29,
    MsgTableDelete = 30,
    MsgDocumentUpdate = 31,
    MsgDocumentReset = 32,
    MsgSignalInvoke = 33,
    MsgMethodReply = 34,
    MsgDocumentInitialized = 35,

    Unknown = 255,
}

/// Discriminant for client message types
#[derive(FromPrimitive, Clone, Copy, Debug)]
pub enum ClientMessages {
    MsgClientIntro = 0,
    MsgClientInvoke = 1,

    Unknown = 255,
}

impl serde::Serialize for ServerMessages {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

/// Archetype of server message
pub enum MessageArchType {
    Create = 0,
    Update = 1,
    Delete = 2,
    Other = 3,
}

impl std::fmt::Display for ServerMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

impl ServerMessages {
    /// Ask what Archetype a given message has
    pub fn arch_type(&self) -> MessageArchType {
        // There is probably a macro that could be made to make my life easier.
        match self {
            ServerMessages::MsgMethodCreate => MessageArchType::Create,
            ServerMessages::MsgMethodDelete => MessageArchType::Delete,
            ServerMessages::MsgSignalCreate => MessageArchType::Create,
            ServerMessages::MsgSignalDelete => MessageArchType::Delete,
            ServerMessages::MsgEntityCreate => MessageArchType::Create,
            ServerMessages::MsgEntityUpdate => MessageArchType::Update,
            ServerMessages::MsgEntityDelete => MessageArchType::Delete,
            ServerMessages::MsgPlotCreate => MessageArchType::Create,
            ServerMessages::MsgPlotUpdate => MessageArchType::Update,
            ServerMessages::MsgPlotDelete => MessageArchType::Delete,
            ServerMessages::MsgBufferCreate => MessageArchType::Create,
            ServerMessages::MsgBufferDelete => MessageArchType::Delete,
            ServerMessages::MsgBufferViewCreate => MessageArchType::Create,
            ServerMessages::MsgBufferViewDelete => MessageArchType::Delete,
            ServerMessages::MsgMaterialCreate => MessageArchType::Create,
            ServerMessages::MsgMaterialUpdate => MessageArchType::Update,
            ServerMessages::MsgMaterialDelete => MessageArchType::Delete,
            ServerMessages::MsgImageCreate => MessageArchType::Create,
            ServerMessages::MsgImageDelete => MessageArchType::Delete,
            ServerMessages::MsgTextureCreate => MessageArchType::Create,
            ServerMessages::MsgTextureDelete => MessageArchType::Delete,
            ServerMessages::MsgSamplerCreate => MessageArchType::Create,
            ServerMessages::MsgSamplerDelete => MessageArchType::Delete,
            ServerMessages::MsgLightCreate => MessageArchType::Create,
            ServerMessages::MsgLightUpdate => MessageArchType::Update,
            ServerMessages::MsgLightDelete => MessageArchType::Delete,
            ServerMessages::MsgGeometryCreate => MessageArchType::Create,
            ServerMessages::MsgGeometryDelete => MessageArchType::Delete,
            ServerMessages::MsgTableCreate => MessageArchType::Create,
            ServerMessages::MsgTableUpdate => MessageArchType::Update,
            ServerMessages::MsgTableDelete => MessageArchType::Delete,
            ServerMessages::MsgDocumentUpdate => MessageArchType::Update,
            ServerMessages::MsgDocumentReset => MessageArchType::Delete,
            _ => MessageArchType::Other,
        }
    }

    /// Asks what component a message operates on
    pub fn component_type(&self) -> ComponentType {
        match self {
            ServerMessages::MsgMethodCreate => ComponentType::Method,
            ServerMessages::MsgMethodDelete => ComponentType::Method,
            ServerMessages::MsgSignalCreate => ComponentType::Signal,
            ServerMessages::MsgSignalDelete => ComponentType::Signal,
            ServerMessages::MsgEntityCreate => ComponentType::Entity,
            ServerMessages::MsgEntityUpdate => ComponentType::Entity,
            ServerMessages::MsgEntityDelete => ComponentType::Entity,
            ServerMessages::MsgPlotCreate => ComponentType::Plot,
            ServerMessages::MsgPlotUpdate => ComponentType::Plot,
            ServerMessages::MsgPlotDelete => ComponentType::Plot,
            ServerMessages::MsgBufferCreate => ComponentType::Buffer,
            ServerMessages::MsgBufferDelete => ComponentType::Buffer,
            ServerMessages::MsgBufferViewCreate => ComponentType::BufferView,
            ServerMessages::MsgBufferViewDelete => ComponentType::BufferView,
            ServerMessages::MsgMaterialCreate => ComponentType::Material,
            ServerMessages::MsgMaterialUpdate => ComponentType::Material,
            ServerMessages::MsgMaterialDelete => ComponentType::Material,
            ServerMessages::MsgImageCreate => ComponentType::Image,
            ServerMessages::MsgImageDelete => ComponentType::Image,
            ServerMessages::MsgTextureCreate => ComponentType::Texture,
            ServerMessages::MsgTextureDelete => ComponentType::Texture,
            ServerMessages::MsgSamplerCreate => ComponentType::Sampler,
            ServerMessages::MsgSamplerDelete => ComponentType::Sampler,
            ServerMessages::MsgLightCreate => ComponentType::Light,
            ServerMessages::MsgLightUpdate => ComponentType::Light,
            ServerMessages::MsgLightDelete => ComponentType::Light,
            ServerMessages::MsgGeometryCreate => ComponentType::Geometry,
            ServerMessages::MsgGeometryDelete => ComponentType::Geometry,
            ServerMessages::MsgTableCreate => ComponentType::Table,
            ServerMessages::MsgTableUpdate => ComponentType::Table,
            ServerMessages::MsgTableDelete => ComponentType::Table,
            ServerMessages::MsgDocumentUpdate => ComponentType::Document,
            ServerMessages::MsgDocumentReset => ComponentType::Document,
            _ => ComponentType::None,
        }
    }
}

pub type NooValueMap = Vec<(value::Value, value::Value)>;

#[derive(Error, Debug)]
pub enum ValueMapLookupError {
    #[error("ID is missing from message map")]
    IDMissing,
}

pub fn lookup<'a>(
    v: &value::Value,
    map: &'a NooValueMap,
) -> Result<&'a value::Value, ValueMapLookupError> {
    for e in map {
        if &e.0 == v {
            return Ok(&e.1);
        }
    }
    Err(ValueMapLookupError::IDMissing)
}

pub fn id_for_message(map: &NooValueMap) -> Option<NooID> {
    let id_name = value::Value::Text(String::from("id"));
    NooID::from_value(lookup(&id_name, map).ok()?)
}

pub mod strings {
    pub const MTHD_TBL_SUBSCRIBE: &str = "noo::tbl_subscribe";
    pub const MTHD_TBL_INSERT: &str = "noo::tbl_insert";
    pub const MTHD_TBL_UPDATE: &str = "noo::tbl_update";
    pub const MTHD_TBL_REMOVE: &str = "noo::tbl_remove";
    pub const MTHD_TBL_CLEAR: &str = "noo::tbl_clear";
    pub const MTHD_TBL_UPDATE_SELECTION: &str = "noo::tbl_update_selection";

    pub const SIG_TBL_RESET: &str = "noo::tbl_reset";
    pub const SIG_TBL_UPDATED: &str = "noo::tbl_updated";
    pub const SIG_TBL_ROWS_REMOVED: &str = "noo::tbl_rows_removed";
    pub const SIG_TBL_SELECTION_UPDATED: &str = "noo::tbl_selection_updated";
    pub const SIG_SIGNAL_ATTENTION: &str = "noo::signal_attention";

    pub const MTHD_ACTIVATE: &str = "noo::activate";
    pub const MTHD_GET_ACTIVATION_CHOICES: &str = "noo::get_activation_choices";
    pub const MTHD_GET_VAR_KEYS: &str = "noo::get_var_keys";
    pub const MTHD_GET_VAR_OPTIONS: &str = "noo::get_var_options";
    pub const MTHD_GET_VAR_VALUE: &str = "noo::get_var_value";
    pub const MTHD_SET_VAR_VALUE: &str = "noo::set_var_value";
    pub const MTHD_SET_POSITION: &str = "noo::set_position";
    pub const MTHD_SET_ROTATION: &str = "noo::set_rotation";
    pub const MTHD_SET_SCALE: &str = "noo::set_scale";
    pub const MTHD_SELECT_REGION: &str = "noo::select_region";
    pub const MTHD_SELECT_SPHERE: &str = "noo::select_sphere";
    pub const MTHD_SELECT_HALF_PLANE: &str = "noo::select_half_plane";
    pub const MTHD_SELECT_HULL: &str = "noo::select_hull";
    pub const MTHD_PROBE_AT: &str = "noo::probe_at";
    pub const MTHD_SIGNAL_ATTENTION: &str = "noo::signal_attention";
    pub const MTHD_CLIENT_VIEW: &str = "noo::client_view";

    pub const TAG_USER_HIDDEN: &str = "noo::user_hidden";

    pub const HINT_ANY: &str = "noo::any";
    pub const HINT_TEXT: &str = "noo::text";
    pub const HINT_INTEGER: &str = "noo::integer";
    pub const HINT_INTEGERLIST: &str = "noo::integerlist";
    pub const HINT_REAL: &str = "noo::real";
    pub const HINT_REALLIST: &str = "noo::reallist";
    pub const HINT_DATA: &str = "noo::data";
    pub const HINT_LIST: &str = "noo::list";
    pub const HINT_MAP: &str = "noo::map";
    pub const HINT_ANYID: &str = "noo::anyid";
    pub const HINT_OBJECTID: &str = "noo::objectid";
    pub const HINT_TABLEID: &str = "noo::tableid";
    pub const HINT_SIGNALID: &str = "noo::signalid";
    pub const HINT_METHODID: &str = "noo::methodid";
    pub const HINT_MATERIALID: &str = "noo::materialid";
    pub const HINT_GEOMETRYID: &str = "noo::geometryid";
    pub const HINT_LIGHTID: &str = "noo::lightid";
    pub const HINT_TEXTUREID: &str = "noo::textureid";
    pub const HINT_BUFFERID: &str = "noo::bufferid";
    pub const HINT_PLOTID: &str = "noo::plotid";
}
