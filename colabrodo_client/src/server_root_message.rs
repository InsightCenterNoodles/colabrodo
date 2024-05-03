//! Code to help extract a stream of messages from the server

use crate::components::*;
use colabrodo_common::{
    common::ServerMessageIDs,
    components::{LightState, LightStateUpdatable},
    nooid::*,
    server_communication::{DocumentInit, DocumentReset, MessageMethodReply},
    types::CommonDeleteMessage,
};
use num_traits::FromPrimitive;
use serde::{de::Visitor, Deserialize, Serialize};

/// A type to represent a message pack from the server
#[derive(Debug)]
pub struct ServerRootMessage {
    pub list: Vec<FromServer>,
}

struct ServerRootMessageVisitor;

fn push_next<'de, A>(
    id: ServerMessageIDs,
    seq: &mut A,
) -> Result<FromServer, A::Error>
where
    A: serde::de::SeqAccess<'de>,
{
    let ret = match id {
        ServerMessageIDs::MsgMethodCreate => FromServer::Method(
            ModMethod::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgMethodDelete => FromServer::Method(
            ModMethod::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgSignalCreate => FromServer::Signal(
            ModSignal::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgSignalDelete => FromServer::Signal(
            ModSignal::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgEntityCreate => FromServer::Entity(
            ModEntity::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgEntityUpdate => FromServer::Entity(
            ModEntity::Update(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgEntityDelete => FromServer::Entity(
            ModEntity::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgPlotCreate => {
            FromServer::Plot(ModPlot::Create(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?))
        }
        ServerMessageIDs::MsgPlotUpdate => {
            FromServer::Plot(ModPlot::Update(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?))
        }
        ServerMessageIDs::MsgPlotDelete => {
            FromServer::Plot(ModPlot::Delete(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?))
        }
        ServerMessageIDs::MsgBufferCreate => FromServer::Buffer(
            ModBuffer::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgBufferDelete => FromServer::Buffer(
            ModBuffer::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgBufferViewCreate => FromServer::BufferView(
            ModBufferView::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgBufferViewDelete => FromServer::BufferView(
            ModBufferView::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgMaterialCreate => FromServer::Material(
            ModMaterial::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgMaterialUpdate => FromServer::Material(
            ModMaterial::Update(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgMaterialDelete => FromServer::Material(
            ModMaterial::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgImageCreate => FromServer::Image(
            ModImage::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgImageDelete => FromServer::Image(
            ModImage::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgTextureCreate => FromServer::Texture(
            ModTexture::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgTextureDelete => FromServer::Texture(
            ModTexture::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgSamplerCreate => FromServer::Sampler(
            ModSampler::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgSamplerDelete => FromServer::Sampler(
            ModSampler::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgLightCreate => FromServer::Light(
            ModLight::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgLightUpdate => FromServer::Light(
            ModLight::Update(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgLightDelete => FromServer::Light(
            ModLight::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgGeometryCreate => FromServer::Geometry(
            ModGeometry::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgGeometryDelete => FromServer::Geometry(
            ModGeometry::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgTableCreate => FromServer::Table(
            ModTable::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgTableUpdate => FromServer::Table(
            ModTable::Update(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgTableDelete => FromServer::Table(
            ModTable::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgDocumentUpdate => {
            FromServer::MsgDocumentUpdate(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?)
        }
        ServerMessageIDs::MsgDocumentReset => {
            FromServer::MsgDocumentReset(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?)
        }
        ServerMessageIDs::MsgSignalInvoke => {
            FromServer::MsgSignalInvoke(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?)
        }
        ServerMessageIDs::MsgMethodReply => {
            FromServer::MsgMethodReply(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?)
        }
        ServerMessageIDs::MsgDocumentInitialized => {
            FromServer::MsgDocumentInitialized(seq.next_element()?.ok_or_else(
                || serde::de::Error::custom("Missing required message"),
            )?)
        }
        ServerMessageIDs::MsgPhysicsCreate => FromServer::Physics(
            ModPhysics::Create(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::MsgPhysicsDelete => FromServer::Physics(
            ModPhysics::Delete(seq.next_element()?.ok_or_else(|| {
                serde::de::Error::custom("Missing required message")
            })?),
        ),
        ServerMessageIDs::Unknown => {
            log::debug!("Unknown ID, bailing");
            return Err(serde::de::Error::custom("Unknown message id"));
        }
    };

    Ok(ret)
}

impl<'de> Visitor<'de> for ServerRootMessageVisitor {
    type Value = ServerRootMessage;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "An interleaved array of id and content")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut ret = ServerRootMessage { list: Vec::new() };

        loop {
            log::debug!("Decoding next message:");
            let id: Option<u32> = seq.next_element()?;

            if id.is_none() {
                log::debug!("ID is None, breaking");
                break;
            }

            log::debug!("ID: {id:?}");

            let id: ServerMessageIDs =
                match <ServerMessageIDs as FromPrimitive>::from_u32(id.unwrap())
                {
                    Some(x) => x,
                    None => break,
                };

            log::debug!("Mapped to: {id:?}");

            match push_next(id, &mut seq) {
                Ok(x) => {
                    if log::log_enabled!(log::Level::Debug) {
                        log::debug!("Found: {x:?}")
                    }
                    ret.list.push(x)
                }
                Err(x) => {
                    log::error!("Unable to deserialize message {id:?} from server: {x:?}");
                    // HACK
                    panic!("NO GOOD");
                }
            }
        }

        Ok(ret)
    }
}

impl<'de> Deserialize<'de> for ServerRootMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ServerRootMessageVisitor)
    }
}

// ============================================================================

/// A message that has an id attached to it
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientCommonTagged<IDType, Nested> {
    pub id: IDType,

    #[serde(flatten)]
    pub content: Nested,
}

/// A message that is method oriented
#[derive(Debug)]
pub enum ModMethod {
    Create(ClientCommonTagged<MethodID, ClientMethodState>),
    Delete(CommonDeleteMessage<MethodID>),
}

/// A message that is signal oriented
#[derive(Debug)]
pub enum ModSignal {
    Create(ClientCommonTagged<SignalID, ClientSignalState>),
    Delete(CommonDeleteMessage<SignalID>),
}

/// A message that is entity oriented
#[derive(Debug)]
pub enum ModEntity {
    Create(ClientCommonTagged<EntityID, ClientEntityState>),
    Update(ClientCommonTagged<EntityID, ClientEntityUpdate>),
    Delete(CommonDeleteMessage<EntityID>),
}

/// A message that is plot oriented
#[derive(Debug)]
pub enum ModPlot {
    Create(ClientCommonTagged<PlotID, ClientPlotState>),
    Update(ClientCommonTagged<PlotID, ClientPlotUpdate>),
    Delete(CommonDeleteMessage<PlotID>),
}

/// A message that is buffer oriented
#[derive(Debug)]
pub enum ModBuffer {
    Create(ClientCommonTagged<BufferID, BufferState>),
    Delete(CommonDeleteMessage<BufferID>),
}

/// A message that is buffer view oriented
#[derive(Debug)]
pub enum ModBufferView {
    Create(ClientCommonTagged<BufferViewID, ClientBufferViewState>),
    Delete(CommonDeleteMessage<BufferViewID>),
}

/// A message that is material oriented
#[derive(Debug)]
pub enum ModMaterial {
    Create(ClientCommonTagged<MaterialID, ClientMaterialState>),
    Update(ClientCommonTagged<MaterialID, ClientMaterialUpdate>),
    Delete(CommonDeleteMessage<MaterialID>),
}

/// A message that is image oriented
#[derive(Debug)]
pub enum ModImage {
    Create(ClientCommonTagged<ImageID, ClientImageState>),
    Delete(CommonDeleteMessage<ImageID>),
}

/// A message that is texture oriented
#[derive(Debug)]
pub enum ModTexture {
    Create(ClientCommonTagged<TextureID, ClientTextureState>),
    Delete(CommonDeleteMessage<TextureID>),
}

/// A message that is sampler oriented
#[derive(Debug)]
pub enum ModSampler {
    Create(ClientCommonTagged<SamplerID, SamplerState>),
    Delete(CommonDeleteMessage<SamplerID>),
}

/// A message that is light oriented
#[derive(Debug)]
pub enum ModLight {
    Create(ClientCommonTagged<LightID, LightState>),
    Update(ClientCommonTagged<LightID, LightStateUpdatable>),
    Delete(CommonDeleteMessage<LightID>),
}

/// A message that is geometry oriented
#[derive(Debug)]
pub enum ModGeometry {
    Create(ClientCommonTagged<GeometryID, ClientGeometryState>),
    Delete(CommonDeleteMessage<GeometryID>),
}

/// A message that is table oriented
#[derive(Debug)]
pub enum ModTable {
    Create(ClientCommonTagged<TableID, ClientTableState>),
    Update(ClientCommonTagged<TableID, ClientTableUpdate>),
    Delete(CommonDeleteMessage<TableID>),
}

/// A message that is table oriented
#[derive(Debug)]
pub enum ModPhysics {
    Create(ClientCommonTagged<PhysicsID, ClientPhysicsState>),
    Delete(CommonDeleteMessage<PhysicsID>),
}

/// An enum to represent all messages from the server.
///
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum FromServer {
    Method(ModMethod),
    Signal(ModSignal),
    Entity(ModEntity),
    Plot(ModPlot),
    Buffer(ModBuffer),
    BufferView(ModBufferView),
    Material(ModMaterial),
    Image(ModImage),
    Texture(ModTexture),
    Sampler(ModSampler),
    Light(ModLight),
    Geometry(ModGeometry),
    Table(ModTable),
    Physics(ModPhysics),
    MsgDocumentUpdate(ClientDocumentUpdate),
    MsgDocumentReset(DocumentReset),
    MsgSignalInvoke(ClientMessageSignalInvoke),
    MsgMethodReply(MessageMethodReply),
    MsgDocumentInitialized(DocumentInit),
}

#[cfg(test)]
mod tests {
    use colabrodo_common::{
        nooid::{BufferID, NooID},
        types::ByteBuff,
    };
    use serde::Serialize;

    use crate::{
        components::BufferState, server_root_message::ClientCommonTagged,
    };

    #[test]
    fn buffer_tagged_state_serde_bytes() {
        type Tagged = ClientCommonTagged<BufferID, BufferState>;

        let rep = BufferState::new_from_bytes(vec![10, 12, 120, 123]);

        let t = Tagged {
            id: BufferID(NooID::new(10, 20)),
            content: rep,
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&t, &mut pack).expect("Pack");

        let other: Tagged = ciborium::de::from_reader(pack.as_slice()).unwrap();

        assert_eq!(t, other);
    }

    #[test]
    fn buffer_tagged_state_serde_url() {
        type Tagged = ClientCommonTagged<BufferID, BufferState>;

        let rep = BufferState::new_from_url("http://wombat.com", 1024);

        let t = Tagged {
            id: BufferID(NooID::new(10, 20)),
            content: rep,
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&t, &mut pack).expect("Pack");

        let other: Tagged = ciborium::de::from_reader(pack.as_slice()).unwrap();

        assert_eq!(t, other);
    }

    #[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
    pub struct LClientCommonTagged {
        pub id: BufferID,

        pub name: Option<String>,

        pub size: u64,

        pub inline_bytes: Option<ByteBuff>,
        pub uri_bytes: Option<url::Url>,
    }

    #[test]
    fn buffer_tagged_state_serde_url2() {
        type Tagged = LClientCommonTagged;

        let t = Tagged {
            id: BufferID(NooID::new(10, 20)),
            name: None,
            size: 1023,
            inline_bytes: None,
            uri_bytes: Some("http://wombat.com".parse().unwrap()),
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&t, &mut pack).expect("Pack");

        let other: Tagged = ciborium::de::from_reader(pack.as_slice()).unwrap();

        assert_eq!(t, other);
    }
}
