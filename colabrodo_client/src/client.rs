use crate::{
    components::*,
    server_root_message::{FromServer, ServerRootMessage},
};
pub use colabrodo_common::client_communication::ClientInvokeMessage;
pub use colabrodo_common::server_communication::MessageMethodReply;
use colabrodo_common::{
    client_communication::{ClientIntroductionMessage, ClientMessageID},
    components::LightState,
};
pub use colabrodo_common::{components::UpdatableWith, nooid::NooID};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, info};
use std::collections::HashMap;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::{tungstenite, MaybeTlsStream};

pub trait ComponentList<State> {
    fn on_create(&mut self, id: NooID, state: State);
    fn on_delete(&mut self, id: NooID);
    fn find(&self, id: &NooID) -> Option<&State>;
}

pub trait UpdatableComponentList<State>: ComponentList<State>
where
    State: UpdatableWith,
{
    fn on_update(&mut self, id: NooID, update: State::Substate);
}

#[derive(Debug)]
pub struct BasicComponentList<State> {
    components: HashMap<NooID, State>,
}

impl<State> Default for BasicComponentList<State> {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

impl<State> ComponentList<State> for BasicComponentList<State> {
    fn on_create(&mut self, id: NooID, state: State) {
        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        self.components.remove(&id);
    }

    fn find(&self, id: &NooID) -> Option<&State> {
        self.components.get(&id)
    }
}

#[derive(Debug)]
pub struct BasicUpdatableList<State> {
    components: HashMap<NooID, State>,
}

impl<State> Default for BasicUpdatableList<State> {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

impl<State> ComponentList<State> for BasicUpdatableList<State> {
    fn on_create(&mut self, id: NooID, state: State) {
        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        self.components.remove(&id);
    }

    fn find(&self, id: &NooID) -> Option<&State> {
        self.components.get(&id)
    }
}

impl<State> UpdatableComponentList<State> for BasicUpdatableList<State>
where
    State: UpdatableWith,
{
    fn on_update(&mut self, id: NooID, update: State::Substate) {
        if let Some(item) = self.components.get_mut(&id) {
            item.update(update);
        }
    }
}

pub trait UserClientState {
    type MethodL: ComponentList<MethodState>;
    type SignalL: ComponentList<SignalState>;

    type BufferL: ComponentList<BufferState>;
    type BufferViewL: ComponentList<ClientBufferViewState>;

    type SamplerL: ComponentList<SamplerState>;
    type ImageL: ComponentList<ClientImageState>;
    type TextureL: ComponentList<ClientTextureState>;

    type MaterialL: UpdatableComponentList<ClientMaterialState>;
    type GeometryL: ComponentList<ClientGeometryState>;

    type LightL: UpdatableComponentList<LightState>;

    type TableL: UpdatableComponentList<ClientTableState>;
    type PlotL: UpdatableComponentList<ClientPlotState>;

    type EntityL: UpdatableComponentList<ClientEntityState>;

    type CommandType;
    type ArgumentType;

    fn new(
        a: Self::ArgumentType,
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self;

    fn method_list(&mut self) -> &mut Self::MethodL;
    fn signal_list(&mut self) -> &mut Self::SignalL;

    fn buffer_list(&mut self) -> &mut Self::BufferL;
    fn buffer_view_list(&mut self) -> &mut Self::BufferViewL;

    fn sampler_list(&mut self) -> &mut Self::SamplerL;
    fn image_list(&mut self) -> &mut Self::ImageL;
    fn texture_list(&mut self) -> &mut Self::TextureL;

    fn material_list(&mut self) -> &mut Self::MaterialL;
    fn geometry_list(&mut self) -> &mut Self::GeometryL;

    fn light_list(&mut self) -> &mut Self::LightL;

    fn table_list(&mut self) -> &mut Self::TableL;
    fn plot_list(&mut self) -> &mut Self::PlotL;

    fn entity_list(&mut self) -> &mut Self::EntityL;

    fn document_update(&mut self, update: ClientDocumentUpdate);

    fn document_reset(&mut self) {}

    fn on_signal_invoke(&mut self, _signal: ClientMessageSignalInvoke) {}
    fn on_method_reply(&mut self, _method_reply: MessageMethodReply) {}
    fn on_document_ready(&mut self) {}

    fn on_command(&mut self, _c: Self::CommandType) {}
}

#[derive(Error, Debug)]
pub enum UserClientNext {
    #[error("Decode error")]
    DecodeError(String),
}

pub fn handle_next<U: UserClientState>(
    state: &mut U,
    message: &[u8],
) -> Result<(), UserClientNext> {
    debug!("Handling next message...");
    let root: ServerRootMessage = ciborium::de::from_reader(message)
        .map_err(|x| UserClientNext::DecodeError(x.to_string()))?;

    for msg in root.list {
        handle_next_message(state, msg)?;
    }

    Ok(())
}

fn handle_next_message<U: UserClientState>(
    state: &mut U,
    m: FromServer,
) -> Result<(), UserClientNext> {
    match m {
        FromServer::MsgMethodCreate(x) => {
            state.method_list().on_create(x.id, x.content);
        }
        FromServer::MsgMethodDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgSignalCreate(x) => {
            state.signal_list().on_create(x.id, x.content)
        }
        FromServer::MsgSignalDelete(x) => state.signal_list().on_delete(x.id),
        FromServer::MsgEntityCreate(x) => {
            state.entity_list().on_create(x.id, x.content)
        }
        FromServer::MsgEntityUpdate(x) => {
            state.entity_list().on_update(x.id, x.content)
        }
        FromServer::MsgEntityDelete(x) => state.entity_list().on_delete(x.id),
        FromServer::MsgPlotCreate(x) => {
            state.plot_list().on_create(x.id, x.content)
        }
        FromServer::MsgPlotUpdate(x) => {
            state.plot_list().on_update(x.id, x.content)
        }
        FromServer::MsgPlotDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgBufferCreate(x) => {
            state.buffer_list().on_create(x.id, x.content)
        }
        FromServer::MsgBufferDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgBufferViewCreate(x) => {
            state.buffer_view_list().on_create(x.id, x.content)
        }
        FromServer::MsgBufferViewDelete(x) => {
            state.method_list().on_delete(x.id)
        }
        FromServer::MsgMaterialCreate(x) => {
            state.material_list().on_create(x.id, x.content)
        }
        FromServer::MsgMaterialUpdate(x) => {
            state.material_list().on_update(x.id, x.content)
        }
        FromServer::MsgMaterialDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgImageCreate(x) => {
            state.image_list().on_create(x.id, x.content)
        }
        FromServer::MsgImageDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgTextureCreate(x) => {
            state.texture_list().on_create(x.id, x.content)
        }
        FromServer::MsgTextureDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgSamplerCreate(x) => {
            state.sampler_list().on_create(x.id, x.content)
        }
        FromServer::MsgSamplerDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgLightCreate(x) => {
            state.light_list().on_create(x.id, x.content)
        }
        FromServer::MsgLightUpdate(x) => {
            state.light_list().on_update(x.id, x.content)
        }
        FromServer::MsgLightDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgGeometryCreate(x) => {
            state.geometry_list().on_create(x.id, x.content)
        }
        FromServer::MsgGeometryDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgTableCreate(x) => {
            state.table_list().on_create(x.id, x.content)
        }
        FromServer::MsgTableUpdate(x) => {
            state.table_list().on_update(x.id, x.content)
        }
        FromServer::MsgTableDelete(x) => state.method_list().on_delete(x.id),
        FromServer::MsgDocumentUpdate(x) => state.document_update(x),
        FromServer::MsgDocumentReset(_) => state.document_reset(),
        FromServer::MsgSignalInvoke(x) => state.on_signal_invoke(x),
        FromServer::MsgMethodReply(x) => state.on_method_reply(x),
        FromServer::MsgDocumentInitialized(_) => state.on_document_ready(),
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum UserClientError {
    #[error("Invalid Host")]
    InvalidHost,

    #[error("Connection Error")]
    ConnectionError(tungstenite::Error),
}

pub enum IncomingMessage<T: UserClientState> {
    NetworkMessage(Vec<u8>),
    Command(T::CommandType),
}

pub enum OutgoingMessage {
    MethodInvoke(ClientInvokeMessage),
}

pub async fn start_client<T>(
    host: String,
    name: String,
    a: T::ArgumentType,
) -> Result<(), UserClientError>
where
    T: UserClientState + 'static,
    T::CommandType: std::marker::Send,
    T::ArgumentType: std::marker::Send,
{
    let url =
        url::Url::parse(&host).map_err(|_| UserClientError::InvalidHost)?;

    info!("Connecting to {url}...");

    let conn_result = connect_async(&url)
        .await
        .map_err(|x| UserClientError::ConnectionError(x))?;

    let (to_client_thread_tx, to_client_thread_rx) = std::sync::mpsc::channel();
    let (from_client_thread_tx, from_client_thread_rx) =
        tokio::sync::mpsc::channel(16);

    let temp_tx_handle = from_client_thread_tx.clone();

    let h1 = std::thread::spawn(move || {
        debug!("Creating client worker thread...");
        client_worker_thread::<T>(to_client_thread_rx, temp_tx_handle, a)
    });

    let (ws_stream, _) = conn_result;

    let (mut socket_tx, socket_rx) = ws_stream.split();

    {
        let content = (
            ClientIntroductionMessage::message_id(),
            ClientIntroductionMessage { client_name: name },
        );

        let mut buffer = Vec::<u8>::new();

        ciborium::ser::into_writer(&content, &mut buffer).unwrap();

        socket_tx.send(Message::Binary(buffer)).await.unwrap();
    }

    tokio::spawn(forward_task(from_client_thread_rx, socket_tx));

    socket_rx
        .for_each(|msg| async {
            let data = msg.unwrap().into_data();

            to_client_thread_tx
                .send(IncomingMessage::NetworkMessage(data))
                .unwrap();
        })
        .await;

    info!("Closing client to {url}...");

    h1.join().unwrap();

    Ok(())
}

async fn forward_task(
    mut input: tokio::sync::mpsc::Receiver<OutgoingMessage>,
    mut output: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) {
    debug!("Starting thread forwarding task");
    while let Some(msg) = input.recv().await {
        let mut buffer = Vec::<u8>::new();

        match msg {
            OutgoingMessage::MethodInvoke(x) => {
                ciborium::ser::into_writer(&x, &mut buffer).unwrap()
            }
        }

        output
            .send(tokio_tungstenite::tungstenite::Message::Binary(buffer))
            .await
            .unwrap();
    }
}

fn client_worker_thread<T>(
    input: std::sync::mpsc::Receiver<IncomingMessage<T>>,
    output: tokio::sync::mpsc::Sender<OutgoingMessage>,
    a: T::ArgumentType,
) where
    T: UserClientState + 'static,
    T::CommandType: std::marker::Send,
    T::ArgumentType: std::marker::Send,
{
    let mut t = T::new(a, output);

    while let Ok(x) = input.recv() {
        match x {
            IncomingMessage::NetworkMessage(bytes) => {
                handle_next(&mut t, bytes.as_slice()).unwrap();
            }
            IncomingMessage::Command(c) => t.on_command(c),
        }
    }
}

//pub fn make_invoke_message
