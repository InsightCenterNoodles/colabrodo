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
use std::{collections::HashMap, fmt::Debug};
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
pub struct BasicComponentList<State>
where
    State: NamedComponent,
{
    name_map: HashMap<String, NooID>,
    components: HashMap<NooID, State>,
}

impl<State> BasicComponentList<State>
where
    State: NamedComponent,
{
    pub fn search_id(&self, name: &str) -> Option<&NooID> {
        self.name_map.get(name)
    }

    pub fn search(&self, name: &str) -> Option<&State> {
        self.find(self.name_map.get(name)?)
    }

    pub fn find_name(&self, id: &NooID) -> Option<&String> {
        self.find(&id)?.name()
    }
}

impl<State> Default for BasicComponentList<State>
where
    State: NamedComponent,
{
    fn default() -> Self {
        Self {
            name_map: HashMap::new(),
            components: HashMap::new(),
        }
    }
}

impl<State> ComponentList<State> for BasicComponentList<State>
where
    State: NamedComponent,
{
    fn on_create(&mut self, id: NooID, state: State) {
        if let Some(n) = state.name() {
            self.name_map.insert(n.clone(), id);
        }

        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        if let Some(n) = self.find_name(&id) {
            let n = n.clone();
            self.name_map.remove(&n);
        }

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

pub trait UserClientState: Debug {
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
    debug!("Handling next message array");
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
    debug!("Handling next message...");
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
    InvalidHost(String),

    #[error("Connection Error")]
    ConnectionError(tungstenite::Error),
}

#[derive(Debug)]
pub enum IncomingMessage<T: UserClientState> {
    NetworkMessage(Vec<u8>),
    Command(T::CommandType),
}

#[derive(Debug)]
pub enum OutgoingMessage {
    Close,
    MethodInvoke(ClientInvokeMessage),
}

pub async fn start_client<T>(
    url: String,
    name: String,
    a: T::ArgumentType,
) -> Result<(), UserClientError>
where
    T: UserClientState + 'static + std::fmt::Debug,
    T::CommandType: std::marker::Send + std::fmt::Debug,
    T::ArgumentType: std::marker::Send,
{
    //let url = url::Url::parse(&host)
    //    .map_err(|x| UserClientError::InvalidHost(x.to_string()))?;

    let (stop_tx, mut stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    info!("Connecting to {url}...");

    let conn_result = connect_async(&url)
        .await
        .map_err(|x| UserClientError::ConnectionError(x))?;

    info!("Connecting to {url}...");

    let (to_client_thread_tx, to_client_thread_rx) = std::sync::mpsc::channel();
    let (from_client_thread_tx, from_client_thread_rx) =
        tokio::sync::mpsc::channel(16);

    let (inter_channel_tx, inter_channel_rx) = tokio::sync::mpsc::channel(16);

    let h1 = std::thread::spawn(move || {
        debug!("Creating client worker thread...");
        client_worker_thread::<T>(
            to_client_thread_rx,
            from_client_thread_tx.clone(),
            a,
        )
    });

    let to_c_handle = tokio::spawn(to_client_task::<T>(
        inter_channel_rx,
        to_client_thread_tx,
        stop_tx.subscribe(),
    ));

    let (ws_stream, _) = conn_result;

    let (mut socket_tx, mut socket_rx) = ws_stream.split();

    {
        let content = (
            ClientIntroductionMessage::message_id(),
            ClientIntroductionMessage { client_name: name },
        );

        let mut buffer = Vec::<u8>::new();

        ciborium::ser::into_writer(&content, &mut buffer).unwrap();

        socket_tx.send(Message::Binary(buffer)).await.unwrap();
    }

    let fhandle = tokio::spawn(forward_task(
        from_client_thread_rx,
        socket_tx,
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            msg = socket_rx.next() => {
                let data = msg.unwrap().unwrap().into_data();

                inter_channel_tx
                    .send(IncomingMessage::NetworkMessage(data))
                    .await
                    .unwrap();
            }
        }
    }

    info!("Closing client to {url}...");

    let join_res = tokio::join!(fhandle, to_c_handle);

    join_res.0.unwrap();
    join_res.1.unwrap();

    h1.join().unwrap();

    Ok(())
}

async fn to_client_task<T>(
    mut input: tokio::sync::mpsc::Receiver<IncomingMessage<T>>,
    output: std::sync::mpsc::Sender<IncomingMessage<T>>,
    mut stopper: tokio::sync::broadcast::Receiver<u8>,
) where
    T: UserClientState + 'static,
{
    debug!("Starting to-client task");
    loop {
        tokio::select! {
            _ = stopper.recv() => break,
            Some(msg) = input.recv() => output.send(msg).unwrap()
        }
    }
    // while let Some(msg) = input.recv().await {
    //     output.send(msg).unwrap();
    // }
    debug!("Ending to-client task");
}

async fn forward_task(
    mut input: tokio::sync::mpsc::Receiver<OutgoingMessage>,
    mut output: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    stopper_tx: tokio::sync::broadcast::Sender<u8>,
    mut stopper: tokio::sync::broadcast::Receiver<u8>,
) {
    debug!("Starting thread forwarding task");

    loop {
        tokio::select! {
            _ = stopper.recv() => break,
            Some(msg) = input.recv() => {
                let mut buffer = Vec::<u8>::new();

                match msg {
                    OutgoingMessage::Close => {
                        output.close().await.unwrap();
                        stopper_tx.send(1).unwrap();
                        break;
                    }
                    OutgoingMessage::MethodInvoke(x) => {
                        let tuple = (ClientInvokeMessage::message_id(), x);
                        ciborium::ser::into_writer(&tuple, &mut buffer).unwrap()
                    }
                }

                output
                    .send(tokio_tungstenite::tungstenite::Message::Binary(buffer))
                    .await
                    .unwrap();
            }
        }
    }
    debug!("Ending thread forwarding task");
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
    debug!("Starting client worker thread");
    let mut t = T::new(a, output);

    while let Ok(x) = input.recv() {
        match x {
            IncomingMessage::NetworkMessage(bytes) => {
                handle_next(&mut t, bytes.as_slice()).unwrap();
            }
            IncomingMessage::Command(c) => t.on_command(c),
        }
    }
    debug!("Ending client worker thread");
}

//pub fn make_invoke_message
