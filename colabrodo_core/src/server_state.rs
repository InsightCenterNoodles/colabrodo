use crate::client_messages::{
    AllClientMessages, ClientInvokeMessage, ClientRootMessage, InvokeIDType,
};
use crate::common::ServerMessages;
use crate::nooid::NooID;
use crate::server_messages::*;
use ciborium::value;
use serde::{ser::SerializeSeq, ser::SerializeStruct, Serialize};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::sync::mpsc::Sender;

pub type CallbackPtr = Sender<Vec<u8>>;

// =============================================================================

pub struct ComponentCell<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    id: NooID,
    host: Rc<RefCell<ServerComponentList<T>>>,
}

impl<T> Debug for ComponentCell<T>
where
    T: Debug + Serialize + ServerStateItemMessageIDs + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentCell")
            .field("id", &self.id)
            .finish()
    }
}

impl<T> Drop for ComponentCell<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn drop(&mut self) {
        println!(
            "Deleting Component {} {}",
            std::any::type_name::<T>(),
            self.id
        );
        // write the delete message
        let write_tuple = (
            T::delete_message_id(),
            crate::server_messages::CommonDeleteMessage { id: self.id },
        );

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

        // now inform the host what has happened

        let mut h = self.host.borrow_mut();

        {
            h.broadcast.send(recorder.data).unwrap();

            h.return_id(self.id);
        }
    }
}

impl<T> ComponentCell<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    pub fn id(&self) -> NooID {
        self.id
    }

    pub fn get(&self) -> ComponentCellGuard<T> {
        ComponentCellGuard {
            guard: self.host.borrow(),
            id: self.id,
        }
        //std::cell::Ref::map(self.host.borrow(), |x| x.get_data(self.id))
    }

    pub fn borrow_mut(&self) -> ComponentCellGuardMut<T> {
        //std::cell::Ref::map(self.host.borrow(), |x| &x.get_data_mut(self.id))
        ComponentCellGuardMut {
            guard: self.host.borrow_mut(),
            id: self.id,
        }
    }

    pub(crate) fn send_to_broadcast(&self, rec: Recorder) {
        self.host.borrow_mut().send_to_tx(rec);
    }
}

pub struct ComponentCellGuard<'a, T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    guard: Ref<'a, ServerComponentList<T>>,
    id: NooID,
}

impl<'a, T> std::ops::Deref for ComponentCellGuard<'a, T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get_data(self.id).unwrap()
    }
}

pub struct ComponentCellGuardMut<'a, T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    guard: RefMut<'a, ServerComponentList<T>>,
    id: NooID,
}

impl<'a, T> std::ops::Deref for ComponentCellGuardMut<'a, T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get_data(self.id).unwrap()
    }
}

impl<'a, T> std::ops::DerefMut for ComponentCellGuardMut<'a, T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.get_data_mut(self.id).unwrap()
    }
}
// =============================================================================

#[derive(Debug)]
pub struct ComponentPtr<T>(pub Rc<ComponentCell<T>>)
where
    T: Serialize + ServerStateItemMessageIDs + Debug;

impl<T> Serialize for ComponentPtr<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.get().serialize(serializer)
    }
}

impl<T> Clone for ComponentPtr<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> ComponentPtr<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    pub fn id(&self) -> NooID {
        self.0.id()
    }

    pub(crate) fn send_to_broadcast(&self, rec: Recorder) {
        self.0.send_to_broadcast(rec)
    }
}

impl<T> Hash for ComponentPtr<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state);
    }
}

impl<T> PartialEq for ComponentPtr<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for ComponentPtr<T> where
    T: Serialize + ServerStateItemMessageIDs + Debug
{
}

// =============================================================================

pub struct ServerComponentList<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    list: HashMap<NooID, T>,
    id_list: HashMap<NooID, Weak<ComponentCell<T>>>,
    broadcast: CallbackPtr,
    free_list: Vec<NooID>,
}

impl<T: Serialize + ServerStateItemMessageIDs + Debug> ServerComponentList<T> {
    fn new(tx: CallbackPtr) -> Self {
        Self {
            list: HashMap::new(),
            id_list: HashMap::new(),
            broadcast: tx,
            free_list: Default::default(),
        }
    }

    fn send_to_tx(&self, rec: Recorder) {
        self.broadcast.send(rec.data).unwrap();
    }

    fn provision_id(&mut self) -> NooID {
        if self.free_list.is_empty() {
            return NooID::new_with_slot(self.list.len().try_into().unwrap());
        }

        NooID::next_generation(self.free_list.pop().unwrap())
    }

    fn return_id(&mut self, id: NooID) {
        self.list.remove(&id);
        self.id_list.remove(&id);
        self.free_list.push(id);
    }

    // Interface
    pub fn new_component(
        &mut self,
        new_t: T,
        host: Rc<RefCell<ServerComponentList<T>>>,
    ) -> ComponentPtr<T> {
        let new_id = self.provision_id();

        println!("Adding Component {} {}", std::any::type_name::<T>(), new_id);

        let write_tuple = (
            T::create_message_id() as u32,
            Bouncer {
                id: new_id,
                content: &new_t,
            },
        );

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

        self.send_to_tx(recorder);

        self.list.insert(new_id, new_t);

        let cell = ComponentCell::<T> {
            id: new_id,
            host: host,
        };

        let cell = Rc::new(cell);

        self.id_list.insert(new_id, Rc::downgrade(&cell));

        ComponentPtr(cell)
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_data(&self, id: NooID) -> Option<&T> {
        self.list.get(&id)
    }

    pub fn get_data_mut(&mut self, id: NooID) -> Option<&mut T> {
        self.list.get_mut(&id)
    }

    pub fn resolve(&self, id: NooID) -> Option<ComponentPtr<T>> {
        match self.id_list.get(&id) {
            None => None,
            Some(x) => match x.upgrade() {
                None => None,
                Some(x) => Some(ComponentPtr(x)),
            },
        }
    }

    fn dump_state_helper<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: SerializeSeq,
    {
        for element in &self.list {
            s.serialize_element(&T::create_message_id())?;
            s.serialize_element(&Bouncer {
                id: *element.0,
                content: &element.1,
            })?;
        }
        Ok(())
    }
}

// =============================================================================

pub struct PubUserCompList<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    list: Rc<RefCell<ServerComponentList<T>>>,
}

impl<T> PubUserCompList<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    fn new(tx: CallbackPtr) -> Self {
        Self {
            list: Rc::new(RefCell::new(ServerComponentList::new(tx))),
        }
    }

    pub fn new_component(&mut self, new_t: T) -> ComponentPtr<T> {
        self.list
            .borrow_mut()
            .new_component(new_t, self.list.clone())
    }

    pub fn resolve(&self, id: NooID) -> Option<ComponentPtr<T>> {
        self.list.borrow().resolve(id)
    }

    pub fn len(&self) -> usize {
        self.list.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn dump_state_helper<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: SerializeSeq,
    {
        self.list.borrow().dump_state_helper(s)
    }
}

// =============================================================================

pub struct ServerState {
    tx: CallbackPtr,
    pub methods: PubUserCompList<MethodState>,
    pub signals: PubUserCompList<SignalState>,

    pub buffers: PubUserCompList<BufferState>,
    pub buffer_views: PubUserCompList<BufferViewState>,

    pub samplers: PubUserCompList<SamplerState>,
    pub images: PubUserCompList<ImageState>,
    pub textures: PubUserCompList<TextureState>,

    pub materials: PubUserCompList<MaterialState>,
    pub geometries: PubUserCompList<GeometryState>,

    pub tables: PubUserCompList<TableState>,
    pub plots: PubUserCompList<PlotState>,

    pub entities: PubUserCompList<EntityState>,

    comm: DocumentUpdate,
}

#[derive(Serialize)]
struct Dummy {
    v: bool,
}

impl Serialize for ServerState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let count = (self.all_element_count() + 2) * 2;
        let mut s = serializer.serialize_seq(Some(count))?;
        self.methods.dump_state_helper(&mut s)?;
        self.signals.dump_state_helper(&mut s)?;
        self.buffers.dump_state_helper(&mut s)?;
        self.buffer_views.dump_state_helper(&mut s)?;
        self.samplers.dump_state_helper(&mut s)?;
        self.images.dump_state_helper(&mut s)?;
        self.textures.dump_state_helper(&mut s)?;
        self.materials.dump_state_helper(&mut s)?;
        self.geometries.dump_state_helper(&mut s)?;
        self.tables.dump_state_helper(&mut s)?;
        self.plots.dump_state_helper(&mut s)?;
        self.entities.dump_state_helper(&mut s)?;

        // custom handling for the doc update.
        s.serialize_element(&ServerMessages::MsgDocumentUpdate)?;
        s.serialize_element(&self.comm)?;

        // now signal full init
        s.serialize_element(&ServerMessages::MsgDocumentInitialized)?;
        s.serialize_element(&Dummy { v: true })?; // dummy content

        s.end()
    }
}

impl ServerState {
    pub fn new(tx: CallbackPtr) -> Self {
        Self {
            tx: tx.clone(),
            methods: PubUserCompList::new(tx.clone()),
            signals: PubUserCompList::new(tx.clone()),
            buffers: PubUserCompList::new(tx.clone()),
            buffer_views: PubUserCompList::new(tx.clone()),
            samplers: PubUserCompList::new(tx.clone()),
            images: PubUserCompList::new(tx.clone()),
            textures: PubUserCompList::new(tx.clone()),
            materials: PubUserCompList::new(tx.clone()),
            geometries: PubUserCompList::new(tx.clone()),
            tables: PubUserCompList::new(tx.clone()),
            plots: PubUserCompList::new(tx.clone()),
            entities: PubUserCompList::new(tx),
            // missing plot and tables
            comm: Default::default(),
        }
    }

    pub fn update_document(&mut self, update: DocumentUpdate) {
        let msg_tuple = (ServerMessages::MsgDocumentUpdate as u32, &update);

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&msg_tuple, &mut recorder.data).unwrap();

        self.tx.send(recorder.data).unwrap();

        self.comm = update;
    }

    pub fn issue_signal(
        &self,
        signals: ComponentPtr<SignalState>,
        context: Option<SignalInvokeObj>,
        arguments: Vec<value::Value>,
    ) {
        let msg_tuple = (
            ServerMessages::MsgDocumentUpdate as u32,
            MessageSignalInvoke {
                id: signals.id(),
                context,
                signal_data: arguments,
            },
        );

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&msg_tuple, &mut recorder.data).unwrap();

        self.tx.send(recorder.data).unwrap();
    }

    // A helper function for serialization
    // returns message count
    fn all_element_count(&self) -> usize {
        self.methods.len()
            + self.signals.len()
            + self.buffers.len()
            + self.buffer_views.len()
            + self.samplers.len()
            + self.images.len()
            + self.textures.len()
            + self.materials.len()
            + self.geometries.len()
            + self.tables.len()
            + self.plots.len()
            + self.entities.len()
    }
}

#[derive(Clone)]
pub enum InvokeObj {
    Document,
    Entity(ComponentPtr<EntityState>),
    Table(ComponentPtr<TableState>),
    Plot(ComponentPtr<PlotState>),
}

pub enum SignalInvokeObj {
    Entity(ComponentPtr<EntityState>),
    Table(ComponentPtr<TableState>),
    Plot(ComponentPtr<PlotState>),
}

impl Serialize for SignalInvokeObj {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("InvokeIDType", 1)?;
        match self {
            SignalInvokeObj::Entity(e) => {
                s.serialize_field("entity", &e.id())?
            }
            SignalInvokeObj::Table(e) => s.serialize_field("table", &e.id())?,
            SignalInvokeObj::Plot(e) => s.serialize_field("plot", &e.id())?,
        }
        s.end()
    }
}

pub type MethodResult = Result<value::Value, MethodException>;

pub trait UserServerState {
    fn new(tx: CallbackPtr) -> Self;
    fn initialize_state(&mut self);
    fn mut_state(&mut self) -> &ServerState;
    fn state(&self) -> &ServerState;
    fn invoke(
        &mut self,
        method: ComponentPtr<MethodState>,
        context: InvokeObj,
        args: Vec<value::Value>,
    ) -> MethodResult;
}

fn find_method_in_state(
    method: &ComponentPtr<MethodState>,
    state: &Option<Vec<ComponentReference<MethodState>>>,
) -> bool {
    match state {
        None => false,
        Some(x) => {
            for m in x {
                if m.id() == method.id() {
                    return true;
                }
            }
            false
        }
    }
}

fn invoke_helper(
    c: &mut impl UserServerState,
    invoke: ClientInvokeMessage,
) -> Result<value::Value, MethodException> {
    let method = c.state().methods.resolve(invoke.method).ok_or_else(|| {
        MethodException {
            code: ExceptionCodes::MethodNotFound as i32,
            ..Default::default()
        }
    })?;

    // get context

    let context = match invoke.context {
        None => Some(InvokeObj::Document),
        Some(id) => match id {
            InvokeIDType::Entity(eid) => {
                c.state().entities.resolve(eid).map(InvokeObj::Entity)
            }
            InvokeIDType::Table(eid) => {
                c.state().tables.resolve(eid).map(InvokeObj::Table)
            }
            InvokeIDType::Plot(eid) => {
                c.state().plots.resolve(eid).map(InvokeObj::Plot)
            }
        },
    };

    let context = context.ok_or_else(|| MethodException {
        code: ExceptionCodes::MethodNotFound as i32,
        ..Default::default()
    })?;

    // make sure the object has the method attached

    let has_method = match &context {
        InvokeObj::Document => {
            find_method_in_state(&method, &c.state().comm.methods_list)
        }
        InvokeObj::Entity(x) => {
            find_method_in_state(&method, &x.0.get().extra.methods_list)
        }
        InvokeObj::Plot(x) => {
            find_method_in_state(&method, &x.0.get().extra.methods_list)
        }
        InvokeObj::Table(x) => {
            find_method_in_state(&method, &x.0.get().extra.methods_list)
        }
    };

    if !has_method {
        return Err(MethodException {
            code: ExceptionCodes::MethodNotFound as i32,
            ..Default::default()
        });
    }

    // all valid. pass along

    c.invoke(method, context, invoke.args)
}

pub fn handle_next<F>(c: &mut impl UserServerState, msg: Vec<u8>, write: F)
where
    F: Fn(Vec<u8>),
{
    // extract a typed message from the input stream
    let root_message = ciborium::de::from_reader(msg.as_slice());

    let root_message: ClientRootMessage = root_message.unwrap();

    for message in root_message.list {
        match message {
            AllClientMessages::Intro(_) => {
                // dump current state to the client. The serde handler should
                // do the right thing here and make one big list of messages
                // to send back
                println!("Client joined, providing initial state");
                let mut recorder = Recorder::default();

                ciborium::ser::into_writer(&c.state(), &mut recorder.data)
                    .unwrap();

                write(recorder.data);
            }
            AllClientMessages::Invoke(invoke) => {
                let reply_id = invoke.invoke_id.clone();

                let result = invoke_helper(c, invoke);

                if let Some(resp) = reply_id {
                    let mut reply = MessageMethodReply {
                        invoke_id: resp,
                        ..Default::default()
                    };

                    match result {
                        Err(x) => {
                            reply.method_exception = Some(x);
                        }
                        Ok(result) => {
                            reply.result = Some(result);
                        }
                    }

                    // now send it back

                    let msg = (ServerMessages::MsgMethodReply, reply);

                    let mut recorder = Recorder::default();

                    ciborium::ser::into_writer(&msg, &mut recorder.data)
                        .unwrap();

                    write(recorder.data);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use ciborium::{cbor, tag::Required, value::Value};

    use super::*;

    fn encode(v: value::Value) -> Vec<u8> {
        let mut ret: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&v, &mut ret).unwrap();

        ret
    }

    fn encode_msg<T: Serialize>(item: &T) -> Vec<u8> {
        let mut ret: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&item, &mut ret).unwrap();

        ret
    }

    fn decode(v: &Vec<u8>) -> Value {
        let ret: Value = ciborium::de::from_reader(v.as_slice()).unwrap();
        ret
    }

    #[test]
    fn build_server_state() {
        // we test by encoding to cbor and then decoding.
        // messages can be encoded different ways, ie, indefinite size of maps, etc.
        let (tx, rx) = std::sync::mpsc::channel();

        let mut state = ServerState::new(tx);

        state
            .buffers
            .new_component(BufferState::new_from_bytes(vec![10, 10, 25, 25]));

        let component_b = state.buffers.new_component(
            BufferState::new_from_url("http://wombat.com", 1024),
        );

        state.buffer_views.new_component(
            BufferViewState::new_from_whole_buffer(component_b.clone()),
        );

        // messages

        let mut messages = VecDeque::new();

        #[derive(Serialize)]
        struct ByteMessage {
            id: NooID,
            size: u32,
            inline_bytes: ByteBuff,
        }

        messages.push_back(encode_msg(&(
            10,
            ByteMessage {
                id: NooID::new(0, 0),
                size: 4,
                inline_bytes: ByteBuff::new(vec![10, 10, 25, 25]),
            },
        )));

        messages.push_back(encode(
            cbor!(
                [
                    11,
                    {
                        "id" => [0, 0],
                    }
                ]
            )
            .unwrap(),
        ));

        #[derive(Serialize)]
        struct ComplexMsg {
            id: NooID,
            size: u32,
            uri_bytes: ciborium::tag::Required<String, 32>,
        }

        let complex_message = (
            10,
            ComplexMsg {
                id: NooID::new(0, 1),
                size: 1024,
                uri_bytes: Required("http://wombat.com".to_string()),
            },
        );

        messages.push_back(encode_msg(&complex_message));

        messages.push_back(encode(
            cbor!(
                [
                    12,
                    {
                        "id" => [0, 0],
                        "source_buffer" => [0, 1],
                        "type"=> "UNK",
                        "offset"=> 0,
                        "length"=> 1024
                    }
                ]
            )
            .unwrap(),
        ));

        messages.push_back(encode(
            cbor!(
                [
                    13,
                    {
                        "id"=> [0, 0]
                    }
                ]
            )
            .unwrap(),
        ));

        while let Ok(msg) = rx.try_recv() {
            //println!("{msg:02X?}");
            let truth = messages.pop_front().unwrap();

            assert_eq!(
                decode(&truth),
                decode(&msg),
                "Messages do not match! Truth: {truth:02X?} | Got: {msg:02X?}"
            );
        }
    }
}
