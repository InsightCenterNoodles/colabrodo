use crate::components::*;
use colabrodo_common::{
    common::ServerMessageIDs,
    components::{LightState, LightStateUpdatable},
    nooid::NooID,
    server_communication::{DocumentInit, DocumentReset, MessageMethodReply},
    types::CommonDeleteMessage,
};
use num_traits::FromPrimitive;
use serde::{de::Visitor, Deserialize};

struct ServerRootMessage {
    list: Vec<FromServer>,
}

struct ServerRootMessageVisitor;

fn push_next<'de, A>(id: ServerMessageIDs, seq: &mut A) -> Option<FromServer>
where
    A: serde::de::SeqAccess<'de>,
{
    match id {
        ServerMessageIDs::MsgMethodCreate => {
            Some(FromServer::MsgMethodCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgMethodDelete => {
            Some(FromServer::MsgMethodDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgSignalCreate => {
            Some(FromServer::MsgSignalCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgSignalDelete => {
            Some(FromServer::MsgSignalDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgEntityCreate => {
            Some(FromServer::MsgEntityCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgEntityUpdate => {
            Some(FromServer::MsgEntityUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgEntityDelete => {
            Some(FromServer::MsgEntityDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgPlotCreate => {
            Some(FromServer::MsgPlotCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgPlotUpdate => {
            Some(FromServer::MsgPlotUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgPlotDelete => {
            Some(FromServer::MsgPlotDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgBufferCreate => {
            Some(FromServer::MsgBufferCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgBufferDelete => {
            Some(FromServer::MsgBufferDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgBufferViewCreate => {
            Some(FromServer::MsgBufferViewCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgBufferViewDelete => {
            Some(FromServer::MsgBufferViewDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgMaterialCreate => {
            Some(FromServer::MsgMaterialCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgMaterialUpdate => {
            Some(FromServer::MsgMaterialUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgMaterialDelete => {
            Some(FromServer::MsgMaterialDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgImageCreate => {
            Some(FromServer::MsgImageCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgImageDelete => {
            Some(FromServer::MsgImageDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgTextureCreate => {
            Some(FromServer::MsgTextureCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgTextureDelete => {
            Some(FromServer::MsgTextureDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgSamplerCreate => {
            Some(FromServer::MsgSamplerCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgSamplerDelete => {
            Some(FromServer::MsgSamplerDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgLightCreate => {
            Some(FromServer::MsgLightCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgLightUpdate => {
            Some(FromServer::MsgLightUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgLightDelete => {
            Some(FromServer::MsgLightDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgGeometryCreate => {
            Some(FromServer::MsgGeometryCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgGeometryDelete => {
            Some(FromServer::MsgGeometryDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgTableCreate => {
            Some(FromServer::MsgTableCreate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgTableUpdate => {
            Some(FromServer::MsgTableUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgTableDelete => {
            Some(FromServer::MsgTableDelete(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgDocumentUpdate => {
            Some(FromServer::MsgDocumentUpdate(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgDocumentReset => {
            Some(FromServer::MsgDocumentReset(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgSignalInvoke => {
            Some(FromServer::MsgSignalInvoke(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgMethodReply => {
            Some(FromServer::MsgMethodReply(seq.next_element().ok()??))
        }
        ServerMessageIDs::MsgDocumentInitialized => Some(
            FromServer::MsgDocumentInitialized(seq.next_element().ok()??),
        ),
        ServerMessageIDs::Unknown => None,
    }
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
            let id: Option<u32> = seq.next_element()?;

            if let Some(id) = id {
                let id: Option<ServerMessageIDs> = FromPrimitive::from_u32(id);

                if let Some(id) = id {
                    push_next(id, &mut seq);
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

pub enum FromServer {
    MsgMethodCreate(MethodState),
    MsgMethodDelete(CommonDeleteMessage),
    MsgSignalCreate(SignalState),
    MsgSignalDelete(CommonDeleteMessage),
    MsgEntityCreate(ClientEntityState),
    MsgEntityUpdate(ClientEntityUpdate),
    MsgEntityDelete(CommonDeleteMessage),
    MsgPlotCreate(ClientPlotState),
    MsgPlotUpdate(ClientPlotUpdate),
    MsgPlotDelete(CommonDeleteMessage),
    MsgBufferCreate(BufferState),
    MsgBufferDelete(CommonDeleteMessage),
    MsgBufferViewCreate(ClientBufferViewState),
    MsgBufferViewDelete(CommonDeleteMessage),
    MsgMaterialCreate(ClientMaterialState),
    MsgMaterialUpdate(ClientMaterialUpdate),
    MsgMaterialDelete(CommonDeleteMessage),
    MsgImageCreate(ClientImageState),
    MsgImageDelete(CommonDeleteMessage),
    MsgTextureCreate(ClientTextureState),
    MsgTextureDelete(CommonDeleteMessage),
    MsgSamplerCreate(SamplerState),
    MsgSamplerDelete(CommonDeleteMessage),
    MsgLightCreate(LightState),
    MsgLightUpdate(LightStateUpdatable),
    MsgLightDelete(CommonDeleteMessage),
    MsgGeometryCreate(ClientGeometryState),
    MsgGeometryDelete(CommonDeleteMessage),
    MsgTableCreate(ClientTableState),
    MsgTableUpdate(ClientTableUpdate),
    MsgTableDelete(CommonDeleteMessage),
    MsgDocumentUpdate(ClientDocumentUpdate),
    MsgDocumentReset(DocumentReset),
    MsgSignalInvoke(ClientMessageSignalInvoke),
    MsgMethodReply(MessageMethodReply),
    MsgDocumentInitialized(DocumentInit),
}
