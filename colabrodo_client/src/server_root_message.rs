use crate::components::*;
use colabrodo_common::{
    common::ServerMessageIDs,
    components::{LightState, LightStateUpdatable},
    nooid,
    server_communication::{DocumentInit, DocumentReset, MessageMethodReply},
    types::CommonDeleteMessage,
};
use num_traits::FromPrimitive;
use serde::{de::Visitor, Deserialize};

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
        ServerMessageIDs::MsgMethodCreate => FromServer::MsgMethodCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgMethodDelete => FromServer::MsgMethodDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgSignalCreate => FromServer::MsgSignalCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgSignalDelete => FromServer::MsgSignalDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgEntityCreate => FromServer::MsgEntityCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgEntityUpdate => FromServer::MsgEntityUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgEntityDelete => FromServer::MsgEntityDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgPlotCreate => FromServer::MsgPlotCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgPlotUpdate => FromServer::MsgPlotUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgPlotDelete => FromServer::MsgPlotDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgBufferCreate => FromServer::MsgBufferCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgBufferDelete => FromServer::MsgBufferDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgBufferViewCreate => {
            FromServer::MsgBufferViewCreate(
                seq.next_element()?
                    .ok_or_else(|| serde::de::Error::custom(""))?,
            )
        }
        ServerMessageIDs::MsgBufferViewDelete => {
            FromServer::MsgBufferViewDelete(
                seq.next_element()?
                    .ok_or_else(|| serde::de::Error::custom(""))?,
            )
        }
        ServerMessageIDs::MsgMaterialCreate => FromServer::MsgMaterialCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgMaterialUpdate => FromServer::MsgMaterialUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgMaterialDelete => FromServer::MsgMaterialDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgImageCreate => FromServer::MsgImageCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgImageDelete => FromServer::MsgImageDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgTextureCreate => FromServer::MsgTextureCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgTextureDelete => FromServer::MsgTextureDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgSamplerCreate => FromServer::MsgSamplerCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgSamplerDelete => FromServer::MsgSamplerDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgLightCreate => FromServer::MsgLightCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgLightUpdate => FromServer::MsgLightUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgLightDelete => FromServer::MsgLightDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgGeometryCreate => FromServer::MsgGeometryCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgGeometryDelete => FromServer::MsgGeometryDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgTableCreate => FromServer::MsgTableCreate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgTableUpdate => FromServer::MsgTableUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgTableDelete => FromServer::MsgTableDelete(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgDocumentUpdate => FromServer::MsgDocumentUpdate(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgDocumentReset => FromServer::MsgDocumentReset(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgSignalInvoke => FromServer::MsgSignalInvoke(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgMethodReply => FromServer::MsgMethodReply(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom(""))?,
        ),
        ServerMessageIDs::MsgDocumentInitialized => {
            FromServer::MsgDocumentInitialized(
                seq.next_element()?
                    .ok_or_else(|| serde::de::Error::custom(""))?,
            )
        }
        ServerMessageIDs::Unknown => {
            log::debug!("Unknown ID, bailing");
            return Err(serde::de::Error::custom(""));
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
                log::debug!("Bad id, breaking");
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
                Ok(x) => ret.list.push(x),
                Err(x) => {
                    log::error!("Unable to deserialize message {id:?} from server: {x:?}");
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

#[derive(Debug, Deserialize)]
pub struct ClientCommonTagged<Nested> {
    pub id: nooid::NooID,

    #[serde(flatten)]
    pub content: Nested,
}

#[allow(clippy::enum_variant_names)] // These all start wtih Msg to match spec
pub enum FromServer {
    MsgMethodCreate(ClientCommonTagged<ClientMethodState>),
    MsgMethodDelete(CommonDeleteMessage),
    MsgSignalCreate(ClientCommonTagged<SignalState>),
    MsgSignalDelete(CommonDeleteMessage),
    MsgEntityCreate(ClientCommonTagged<ClientEntityState>),
    MsgEntityUpdate(ClientCommonTagged<ClientEntityUpdate>),
    MsgEntityDelete(CommonDeleteMessage),
    MsgPlotCreate(ClientCommonTagged<ClientPlotState>),
    MsgPlotUpdate(ClientCommonTagged<ClientPlotUpdate>),
    MsgPlotDelete(CommonDeleteMessage),
    MsgBufferCreate(ClientCommonTagged<BufferState>),
    MsgBufferDelete(CommonDeleteMessage),
    MsgBufferViewCreate(ClientCommonTagged<ClientBufferViewState>),
    MsgBufferViewDelete(CommonDeleteMessage),
    MsgMaterialCreate(ClientCommonTagged<ClientMaterialState>),
    MsgMaterialUpdate(ClientCommonTagged<ClientMaterialUpdate>),
    MsgMaterialDelete(CommonDeleteMessage),
    MsgImageCreate(ClientCommonTagged<ClientImageState>),
    MsgImageDelete(CommonDeleteMessage),
    MsgTextureCreate(ClientCommonTagged<ClientTextureState>),
    MsgTextureDelete(CommonDeleteMessage),
    MsgSamplerCreate(ClientCommonTagged<SamplerState>),
    MsgSamplerDelete(CommonDeleteMessage),
    MsgLightCreate(ClientCommonTagged<LightState>),
    MsgLightUpdate(ClientCommonTagged<LightStateUpdatable>),
    MsgLightDelete(ClientCommonTagged<CommonDeleteMessage>),
    MsgGeometryCreate(ClientCommonTagged<ClientGeometryState>),
    MsgGeometryDelete(CommonDeleteMessage),
    MsgTableCreate(ClientCommonTagged<ClientTableState>),
    MsgTableUpdate(ClientCommonTagged<ClientTableUpdate>),
    MsgTableDelete(CommonDeleteMessage),
    MsgDocumentUpdate(ClientDocumentUpdate),
    MsgDocumentReset(DocumentReset),
    MsgSignalInvoke(ClientMessageSignalInvoke),
    MsgMethodReply(MessageMethodReply),
    MsgDocumentInitialized(DocumentInit),
}
