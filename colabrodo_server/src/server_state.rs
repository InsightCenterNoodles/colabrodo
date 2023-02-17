//! Methods and structs to manage the state of a server. This includes lists of objects, and document methods/signals.
//!
//! Users should define a struct that conforms to the [`UserServerState`] trait; they can then use [`handle_next`] to drive changes to the state.
//!
//! Servers currently require a sink for broadcast messages that should be sent to all clients. Users of this library should provide such a sink (currently [`std::sync::mpsc::Sender<T>`]) and drain it regularily.
//!
//!

use crate::client_messages::{
    AllClientMessages, ClientInvokeMessage, ClientRootMessage, InvokeIDType,
};
use crate::server_messages::*;
use ciborium::value;
use colabrodo_common::common::ServerMessageIDs;
use colabrodo_common::nooid::NooID;
use indexmap::IndexMap;
use serde::{ser::SerializeSeq, ser::SerializeStruct, Serialize};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::{Rc, Weak};
use std::sync::mpsc::Sender;
use thiserror::Error;

pub type CallbackPtr = Sender<Vec<u8>>;

// =============================================================================

/// A struct to manage the lifetime of a component. Holds the id of the component, and the list that contains it.
/// When this struct goes out of scope, the ID is deleted for recycling and the component is erased.
/// Thus, this should be held in an Rc.
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
        log::debug!(
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

        // now inform the host list what has happened
        // this is tricky if, through our delete, we happen to trigger another delete from this list. so we want to move things out first, to release our borrow on the host. We can then let the T go out of scope outside of that borrow.
        let mut _holder: Option<T>;

        {
            let mut h = self.host.borrow_mut();
            h.broadcast.send(recorder.data).unwrap();
            _holder = h.return_id(self.id);
        }

        _holder = None;
    }
}

impl<T> ComponentCell<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    /// Obtain the ID of the managed component
    pub fn id(&self) -> NooID {
        self.id
    }

    /// Obtain the component. This is done in a somewhat roundabout way,
    /// As the containing list is a mutable shared item.
    pub fn get(&self) -> ComponentCellGuard<T> {
        ComponentCellGuard {
            guard: self.host.borrow(),
            id: self.id,
        }
        //std::cell::Ref::map(self.host.borrow(), |x| x.get_data(self.id))
    }

    /// Obtain the component for mutation. Should not be called by users directly, only for our updating infrastructure.
    pub(crate) fn borrow_mut(&self) -> ComponentCellGuardMut<T> {
        //std::cell::Ref::map(self.host.borrow(), |x| &x.get_data_mut(self.id))
        ComponentCellGuardMut {
            guard: self.host.borrow_mut(),
            id: self.id,
        }
    }

    /// Send a given message to the server broadcast pipeline
    pub(crate) fn send_to_broadcast(&self, rec: Recorder) {
        self.host.borrow_mut().send_to_broadcast(rec);
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

/// A shared pointer to a component. This is an internal only type.
/// It is used primarily to make a distinction when serializing lists
/// of component pointers: this will serialize the actual component content.
#[derive(Debug)]
struct ComponentPtr<T>(Rc<ComponentCell<T>>)
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

// =============================================================================

/// A list of components. Has the list of content, and a list of id managed pointers.
/// We use a split list here to reduce the amount of RefCells we would otherwise be throwing around.
/// Also included is a sink for broadcast messages, and a list of free ids for recycling.
struct ServerComponentList<T>
where
    T: Serialize + ServerStateItemMessageIDs + Debug,
{
    // this list HAS to be sorted at the moment.
    // as in most cases parent object IDs are lower in the slot list.
    // however, this is not guaranteed, and some clients scream if it is not
    // true.
    list: IndexMap<NooID, T>,
    id_list: HashMap<NooID, Weak<ComponentCell<T>>>,
    broadcast: CallbackPtr,
    free_list: Vec<NooID>,
}

impl<T: Serialize + ServerStateItemMessageIDs + Debug> ServerComponentList<T> {
    fn new(tx: CallbackPtr) -> Self {
        Self {
            list: IndexMap::new(),
            id_list: HashMap::new(),
            broadcast: tx,
            free_list: Default::default(),
        }
    }

    /// Send a CBOR message to the broadcast sink
    fn send_to_broadcast(&self, rec: Recorder) {
        self.broadcast.send(rec.data).unwrap();
    }

    /// Obtain a new id. Either generates a new ID if there are no free slots. If there are free slots, reuse and bump the generation.
    fn provision_id(&mut self) -> NooID {
        if self.free_list.is_empty() {
            return NooID::new_with_slot(self.list.len().try_into().unwrap());
        }

        NooID::next_generation(self.free_list.pop().unwrap())
    }

    /// Inform us that the given ID (slot) is free to be used again.
    /// This returns the component we destroyed.
    ///
    /// The reason for this is if we delete an item from a list that triggers (through rc ptrs) a delete from the same list. We need to move the reference out of here so calling code can safely release their borrow before dropping again.
    fn return_id(&mut self, id: NooID) -> Option<T> {
        self.id_list.remove(&id);
        self.free_list.push(id);
        self.list.remove(&id)
    }

    // Create a new component. User provides initial state, and we need a pointer to the list (ourselves) to hand out to the new component.
    fn new_component(
        &mut self,
        new_t: T,
        host: Rc<RefCell<ServerComponentList<T>>>,
    ) -> ComponentReference<T> {
        let new_id = self.provision_id();

        log::info!(
            "Adding Component {} {}",
            std::any::type_name::<T>(),
            new_id
        );

        // Use a pack here to encode the message id and the message.
        // TODO: Figure out a way we can pack many messages into one.
        let write_tuple = (
            T::create_message_id() as u32,
            Bouncer {
                id: new_id,
                content: &new_t,
            },
        );

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

        self.send_to_broadcast(recorder);

        // After broadcast, actually insert the content
        self.list.insert(new_id, new_t);

        let cell = ComponentCell::<T> { id: new_id, host };

        let cell = Rc::new(cell);

        self.id_list.insert(new_id, Rc::downgrade(&cell));

        ComponentReference(cell)
    }

    /// Obtain the count of components in this list
    fn len(&self) -> usize {
        self.list.len()
    }

    /// Get a reference to a component by ID
    fn get_data(&self, id: NooID) -> Option<&T> {
        self.list.get(&id)
    }

    /// Get a mutable reference to a component by ID
    fn get_data_mut(&mut self, id: NooID) -> Option<&mut T> {
        self.list.get_mut(&id)
    }

    /// Get a pointer to a component by ID
    fn resolve(&self, id: NooID) -> Option<ComponentReference<T>> {
        match self.id_list.get(&id) {
            None => None,
            Some(x) => x.upgrade().map(ComponentReference::new),
        }
    }

    /// Function that only exists to help serialize all items in this list
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

/// User facing list of components
/// Internally this is a shared pointer to our internal list, as our components need a reference to this list as well.
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
    /// Create a new list
    fn new(tx: CallbackPtr) -> Self {
        Self {
            list: Rc::new(RefCell::new(ServerComponentList::new(tx))),
        }
    }

    /// Create a new component. Initial state for the component should be provided, and in return a reference to the new component is given.
    pub fn new_component(&mut self, new_t: T) -> ComponentReference<T> {
        self.list
            .borrow_mut()
            .new_component(new_t, self.list.clone())
    }

    /// Discover a component by its ID. If the ID is invalid, returns None
    pub fn resolve(&self, id: NooID) -> Option<ComponentReference<T>> {
        self.list.borrow().resolve(id)
    }

    /// Ask how many components are in this list
    pub fn len(&self) -> usize {
        self.list.borrow().len()
    }

    /// Ask if the list is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// This function only exists to help serialize state
    fn dump_state_helper<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: SerializeSeq,
    {
        self.list.borrow().dump_state_helper(s)
    }
}

// =============================================================================

/// Core server state, or Document. Maintains a list of all components that clients may discover. Also maintains state for document lists.
/// See examples for usage.
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

/// A dummy struct for use when we need a message with no content. Terrible.
#[derive(Serialize)]
struct Dummy {
    v: bool,
}

impl Serialize for ServerState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // We would like to be kind here and provide a count of elements
        // the count is the number of all components, plus two messages (document update, and init one) at the end. Then we multiply by 2 as we have IDs for each message
        let count = (self.all_element_count() + 2) * 2;
        let mut s = serializer.serialize_seq(Some(count))?;

        // We should now dump each list in an order that should not cause undefined references or IDs that point to nothing.
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
        s.serialize_element(&ServerMessageIDs::MsgDocumentUpdate)?;
        s.serialize_element(&self.comm)?;

        // now signal full init
        s.serialize_element(&ServerMessageIDs::MsgDocumentInitialized)?;
        s.serialize_element(&Dummy { v: true })?; // dummy content

        s.end()
    }
}

impl ServerState {
    /// Create a new server state. A sink must be provided; this sink will collect messages that are to be broadcasted to all clients, and should be serviced regularly.
    pub fn new(broadcast: CallbackPtr) -> Self {
        Self {
            tx: broadcast.clone(),

            methods: PubUserCompList::new(broadcast.clone()),
            signals: PubUserCompList::new(broadcast.clone()),
            buffers: PubUserCompList::new(broadcast.clone()),
            buffer_views: PubUserCompList::new(broadcast.clone()),
            samplers: PubUserCompList::new(broadcast.clone()),
            images: PubUserCompList::new(broadcast.clone()),
            textures: PubUserCompList::new(broadcast.clone()),
            materials: PubUserCompList::new(broadcast.clone()),
            geometries: PubUserCompList::new(broadcast.clone()),
            tables: PubUserCompList::new(broadcast.clone()),
            plots: PubUserCompList::new(broadcast.clone()),
            entities: PubUserCompList::new(broadcast),

            comm: Default::default(),
        }
    }

    /// Update the document's methods and signals
    pub fn update_document(&mut self, update: DocumentUpdate) {
        let msg_tuple = (ServerMessageIDs::MsgDocumentUpdate as u32, &update);

        let mut recorder = Recorder::default();

        ciborium::ser::into_writer(&msg_tuple, &mut recorder.data).unwrap();

        self.tx.send(recorder.data).unwrap();

        self.comm = update;
    }

    /// Issue a signal for all clients
    ///
    /// Takes a signal to issue, the context on which the signal operates, and the arguments to be sent
    ///
    /// # Panics
    /// This will panic if the broadcast queue is unable to accempt more content.
    pub fn issue_signal(
        &self,
        signals: ComponentReference<SignalState>,
        context: Option<SignalInvokeObj>,
        arguments: Vec<value::Value>,
    ) {
        let msg_tuple = (
            ServerMessageIDs::MsgDocumentUpdate as u32,
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

    /// A helper function for serialization, returns the count of all components
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

/// Helper enum of the target of a method invocation
#[derive(Clone)]
pub enum InvokeObj {
    Document,
    Entity(ComponentReference<EntityState>),
    Table(ComponentReference<TableState>),
    Plot(ComponentReference<PlotState>),
}

/// Helper enum to describe the target of a signal invocation
pub enum SignalInvokeObj {
    Entity(ComponentReference<EntityState>),
    Table(ComponentReference<TableState>),
    Plot(ComponentReference<PlotState>),
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

/// The result of a method invocation.
///
/// Invoked methods can use this to provide a meaningful result, or to signal that an exception has occurred.
pub type MethodResult = Result<value::Value, MethodException>;

/// Trait for operating on user-defined server states.
///
/// Users should have their own struct that conforms to this specification, as it is required for message handling (see [`handle_next`]).
pub trait UserServerState {
    fn mut_state(&mut self) -> &ServerState;
    fn state(&self) -> &ServerState;
    fn invoke(
        &mut self,
        method: ComponentReference<MethodState>,
        context: InvokeObj,
        args: Vec<value::Value>,
    ) -> MethodResult;
}

/// Helper function to determine if a method is indeed attached to a given target.
fn find_method_in_state(
    method: &ComponentReference<MethodState>,
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

/// Helper function to actually invoke a method
///
/// Determines if the method exists, can be invoked on the target, etc, and if so, dispatches to the user server
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

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Decode error")]
    DecodeError(String),
    #[error("Root message is not valid")]
    InvalidRootMessage(String),
}

/// Drive state changes by handling the next message to the server.
///
/// This function takes a user server state, a message, and a writeback function. The message is assumed to be encoded in CBOR. If a specific message is needed to be sent back, the `write` function argument will be called with the content; it is up to user code to determine how to send that to the client.
pub fn handle_next<F>(
    c: &mut impl UserServerState,
    msg: Vec<u8>,
    write: F,
) -> Result<(), ServerError>
where
    F: Fn(Vec<u8>),
{
    // extract a typed message from the input stream
    let root_message = ciborium::de::from_reader(msg.as_slice());

    let root_message: ClientRootMessage = root_message
        .map_err(|_| ServerError::InvalidRootMessage("Unable to extract root client message; this should just be a CBOR array.".to_string()))?;

    for message in root_message.list {
        match message {
            AllClientMessages::Intro(_) => {
                // dump current state to the client. The serde handler should
                // do the right thing here and make one big list of messages
                // to send back
                log::debug!("Client joined, providing initial state");
                let mut recorder = Recorder::default();

                ciborium::ser::into_writer(&c.state(), &mut recorder.data)
                    .unwrap();

                write(recorder.data);
            }
            AllClientMessages::Invoke(invoke) => {
                // copy the reply ident
                let reply_id = invoke.invoke_id.clone();

                // invoke the method and get the result or error
                let result = invoke_helper(c, invoke);

                // if we have a reply id, then we can ship a response. Otherwise, we just skip this step.
                if let Some(resp) = reply_id {
                    // Format a reply object
                    let mut reply = MessageMethodReply {
                        invoke_id: resp,
                        ..Default::default()
                    };

                    // only fill in a certain field if a reply or exception...
                    match result {
                        Err(x) => {
                            reply.method_exception = Some(x);
                        }
                        Ok(result) => {
                            reply.result = Some(result);
                        }
                    }

                    // now send it back
                    let msg = (ServerMessageIDs::MsgMethodReply, reply);

                    let mut recorder = Recorder::default();

                    ciborium::ser::into_writer(&msg, &mut recorder.data)
                        .unwrap();

                    write(recorder.data);
                }
            }
        }
    }

    Ok(())
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

    #[test]
    fn cascade_delete() {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut state = ServerState::new(tx);

        {
            let a = state.entities.new_component(EntityState {
                name: Some("A".to_string()),
                extra: Default::default(),
            });

            let b = state.entities.new_component(EntityState {
                name: Some("B".to_string()),
                extra: EntityStateUpdatable {
                    parent: Some(a),
                    ..Default::default()
                },
            });

            let _c = state.entities.new_component(EntityState {
                name: Some("C".to_string()),
                extra: EntityStateUpdatable {
                    parent: Some(b),
                    ..Default::default()
                },
            });
        }

        while let Ok(_msg) = rx.try_recv() {}
    }
}
