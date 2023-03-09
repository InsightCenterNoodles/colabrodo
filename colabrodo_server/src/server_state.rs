//! Methods and structs to manage the state of a server. This includes lists of objects, and document methods/signals.
//!
//! Users should define a struct that conforms to the [`UserServerState`] trait; they can then use [`handle_next`] to drive changes to the state.
//!
//! Servers currently require a sink for broadcast messages that should be sent to all clients. Users of this library should provide such a sink (currently [`std::sync::mpsc::Sender<T>`]) and drain it regularily.
//!
//!

use crate::server::ClientRecord;
use crate::server_messages::*;
use ciborium::value;
pub use colabrodo_common::common::{
    ComponentType, MessageArchType, ServerMessageIDs,
};
use colabrodo_common::components::*;
use colabrodo_common::nooid::NooID;
pub use colabrodo_common::server_communication::{
    ExceptionCodes, MessageMethodReply, MessageSignalInvoke, MethodException,
    SignalInvokeObj,
};
use indexmap::IndexMap;
use serde::{ser::SerializeSeq, Serialize};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{self, Sender};

#[derive(Debug, Clone)]
pub enum Output {
    Broadcast(Vec<u8>),
    Shutdown,
}

pub type CallbackPtr = Sender<Output>;

// =============================================================================

/// A struct to manage the lifetime of a component. Holds the id of the component, and the list that contains it.
/// When this struct goes out of scope, the ID is deleted for recycling and the component is erased.
/// Thus, this should be held in an Rc.
pub struct ComponentCell<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    id: NooID,
    host: Rc<RefCell<ServerComponentList<T>>>,
}

impl<T> Debug for ComponentCell<T>
where
    T: Debug + Serialize + ComponentMessageIDs + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentCell")
            .field("id", &self.id)
            .finish()
    }
}

impl<T> Drop for ComponentCell<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    fn drop(&mut self) {
        log::debug!(
            "Deleting Component {} {}",
            std::any::type_name::<T>(),
            self.id
        );

        // now inform the host list what has happened
        // this is tricky if, through our delete, we happen to trigger another delete from this list. so we want to move things out first, to release our borrow on the host. We can then let the T go out of scope outside of that borrow.
        let mut _holder: Option<T>;

        {
            let mut h = self.host.borrow_mut();
            _holder = h.return_id(self.id);
        }

        _holder = None;
    }
}

impl<T> ComponentCell<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
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
    T: Serialize + ComponentMessageIDs + Debug,
{
    guard: Ref<'a, ServerComponentList<T>>,
    id: NooID,
}

impl<'a, T> std::ops::Deref for ComponentCellGuard<'a, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get_data(self.id).unwrap()
    }
}

pub struct ComponentCellGuardMut<'a, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    guard: RefMut<'a, ServerComponentList<T>>,
    id: NooID,
}

impl<'a, T> std::ops::Deref for ComponentCellGuardMut<'a, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get_data(self.id).unwrap()
    }
}

impl<'a, T> std::ops::DerefMut for ComponentCellGuardMut<'a, T>
where
    T: Serialize + ComponentMessageIDs + Debug,
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
    T: Serialize + ComponentMessageIDs + Debug;

impl<T> Serialize for ComponentPtr<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
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
    T: Serialize + ComponentMessageIDs + Debug,
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
    T: Serialize + ComponentMessageIDs + Debug,
{
    // this list HAS to be sorted at the moment.
    // as in most cases parent object IDs are lower in the slot list.
    // however, this is not guaranteed, and some clients scream if it is not
    // true.
    list: IndexMap<NooID, T>,
    id_list: HashMap<NooID, Weak<ComponentCell<T>>>,
    broadcast: CallbackPtr,
    free_list: Vec<NooID>,
    owned_list: Vec<Rc<ComponentCell<T>>>,
}

impl<T: Serialize + ComponentMessageIDs + Debug> ServerComponentList<T> {
    fn new(tx: CallbackPtr) -> Self {
        Self {
            list: IndexMap::new(),
            id_list: HashMap::new(),
            broadcast: tx,
            free_list: Default::default(),
            owned_list: Default::default(),
        }
    }

    /// Send a CBOR message to the broadcast sink
    fn send_to_broadcast(&self, rec: Recorder) {
        let _ret = self.broadcast.send(Output::Broadcast(rec.data));
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
        // write the delete message
        let recorder = Recorder::record(
            T::delete_message_id() as u32,
            &colabrodo_common::types::CommonDeleteMessage { id },
        );

        // not sending a message could just mean that the broadcast pipe has been shut down, so we ignore it
        let _err = self.broadcast.send(Output::Broadcast(recorder.data));

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

        let recorder = Recorder::record(
            T::create_message_id() as u32,
            &Bouncer {
                id: new_id,
                content: &new_t,
            },
        );

        self.send_to_broadcast(recorder);

        // After broadcast, actually insert the content
        self.list.insert(new_id, new_t);

        let cell = ComponentCell::<T> { id: new_id, host };

        let cell = Rc::new(cell);

        self.id_list.insert(new_id, Rc::downgrade(&cell));

        ComponentReference(cell)
    }

    fn new_owned_component(
        &mut self,
        new_t: T,
        host: Rc<RefCell<ServerComponentList<T>>>,
    ) -> ComponentReference<T> {
        let ret = self.new_component(new_t, host);
        self.owned_list.push(ret.0.clone());
        ret
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

    fn inspect(&self, id: NooID) -> Option<&T> {
        self.list.get(&id)
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
    T: Serialize + ComponentMessageIDs + Debug,
{
    list: Rc<RefCell<ServerComponentList<T>>>,
}

impl<T> PubUserCompList<T>
where
    T: Serialize + ComponentMessageIDs + Debug,
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

    pub fn new_owned_component(&mut self, new_t: T) -> ComponentReference<T> {
        self.list
            .borrow_mut()
            .new_owned_component(new_t, self.list.clone())
    }

    /// Discover a component by its ID. If the ID is invalid, returns None
    pub fn resolve(&self, id: NooID) -> Option<ComponentReference<T>> {
        self.list.borrow().resolve(id)
    }

    /// Inspect the contents of a component
    pub fn inspect<F, R>(&self, id: NooID, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        self.list.borrow().inspect(id).map(f)
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

    pub methods: PubUserCompList<ServerMethodState>,
    pub signals: PubUserCompList<ServerSignalState>,

    pub buffers: PubUserCompList<BufferState>,
    pub buffer_views: PubUserCompList<ServerBufferViewState>,

    pub samplers: PubUserCompList<SamplerState>,
    pub images: PubUserCompList<ServerImageState>,
    pub textures: PubUserCompList<ServerTextureState>,

    pub materials: PubUserCompList<ServerMaterialState>,
    pub geometries: PubUserCompList<ServerGeometryState>,

    pub tables: PubUserCompList<ServerTableState>,
    pub plots: PubUserCompList<ServerPlotState>,

    pub entities: PubUserCompList<ServerEntityState>,

    pub(crate) comm: ServerDocumentUpdate,

    pub(crate) active_client_info: HashMap<uuid::Uuid, ClientRecord>,
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
    /// Create a new server state.
    pub fn new() -> Arc<Mutex<Self>> {
        let (bcast_send, _) = broadcast::channel(16);

        Arc::new(Mutex::new(Self {
            tx: bcast_send.clone(),

            methods: PubUserCompList::new(bcast_send.clone()),
            signals: PubUserCompList::new(bcast_send.clone()),
            buffers: PubUserCompList::new(bcast_send.clone()),
            buffer_views: PubUserCompList::new(bcast_send.clone()),
            samplers: PubUserCompList::new(bcast_send.clone()),
            images: PubUserCompList::new(bcast_send.clone()),
            textures: PubUserCompList::new(bcast_send.clone()),
            materials: PubUserCompList::new(bcast_send.clone()),
            geometries: PubUserCompList::new(bcast_send.clone()),
            tables: PubUserCompList::new(bcast_send.clone()),
            plots: PubUserCompList::new(bcast_send.clone()),
            entities: PubUserCompList::new(bcast_send),

            comm: Default::default(),

            active_client_info: Default::default(),
        }))
    }

    pub fn new_broadcast_recv(&self) -> broadcast::Receiver<Output> {
        self.tx.subscribe()
    }

    pub fn new_broadcast_send(&self) -> broadcast::Sender<Output> {
        self.tx.clone()
    }

    pub fn shutdown(&self) {
        log::debug!("Server attempting shutdown...");
        self.tx.send(Output::Shutdown).unwrap();
    }

    /// Update the document's methods and signals
    pub fn update_document(&mut self, update: ServerDocumentUpdate) {
        let recorder = Recorder::record(
            ServerMessageIDs::MsgDocumentUpdate as u32,
            &update,
        );

        let _ret = self.tx.send(Output::Broadcast(recorder.data));

        self.comm = update;
    }

    /// Issue a signal for all clients
    ///
    /// Takes a signal to issue, the context on which the signal operates, and the arguments to be sent
    ///
    /// # Panics
    /// This will panic if the broadcast queue is unable to accept more content.
    pub fn issue_signal(
        &self,
        signals: &ComponentReference<ServerSignalState>,
        context: Option<ServerSignalInvokeObj>,
        arguments: Vec<value::Value>,
    ) {
        let recorder = Recorder::record(
            ServerMessageIDs::MsgSignalInvoke as u32,
            &MessageSignalInvoke {
                id: signals.id(),
                context,
                signal_data: arguments,
            },
        );

        self.tx.send(Output::Broadcast(recorder.data)).unwrap();
    }

    pub fn get_client_info(&self, id: uuid::Uuid) -> Option<&ClientRecord> {
        self.active_client_info.get(&id)
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

pub type ServerStatePtr = Arc<Mutex<ServerState>>;

/// Helper enum of the target of a method invocation
#[derive(Clone)]
pub enum InvokeObj {
    Document,
    Entity(ComponentReference<ServerEntityState>),
    Table(ComponentReference<ServerTableState>),
    Plot(ComponentReference<ServerPlotState>),
}

/// Helper enum to describe the target of a signal invocation
pub type ServerSignalInvokeObj = SignalInvokeObj<
    ComponentReference<ServerEntityState>,
    ComponentReference<ServerTableState>,
    ComponentReference<ServerPlotState>,
>;

pub type ServerMessageSignalInvoke = MessageSignalInvoke<
    ComponentReference<ServerEntityState>,
    ComponentReference<ServerTableState>,
    ComponentReference<ServerPlotState>,
>;

/// The result of a method invocation.
///
/// Invoked methods can use this to provide a meaningful result, or to signal that an exception has occurred.
pub type MethodResult = Result<Option<value::Value>, MethodException>;

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use ciborium::{cbor, value::Value};
    use colabrodo_common::types::ByteBuff;

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
        let state = ServerState::new();

        let mut state_lock = state.lock().unwrap();

        let mut recv = state_lock.new_broadcast_recv();

        state_lock
            .buffers
            .new_component(BufferState::new_from_bytes(vec![10, 10, 25, 25]));

        let component_b = state_lock.buffers.new_component(
            BufferState::new_from_url("http://wombat.com", 1024),
        );

        state_lock
            .buffer_views
            .new_component(BufferViewState::new_from_whole_buffer(component_b));

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
            uri_bytes: String,
        }

        let complex_message = (
            10,
            ComplexMsg {
                id: NooID::new(0, 1),
                size: 1024,
                uri_bytes: "http://wombat.com/".to_string(),
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

        messages.push_back(encode(
            cbor!(
                [
                    11,
                    {
                        "id" => [0, 1],
                    }
                ]
            )
            .unwrap(),
        ));

        while let Ok(msg) = recv.try_recv() {
            //println!("{msg:02X?}");

            let msg = match msg {
                Output::Broadcast(x) => x,
                _ => panic!("Wrong message"),
            };

            println!("GOT: {:?}", decode(&msg));

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
        let state = ServerState::new();

        let mut state_lock = state.lock().unwrap();

        let mut recv = state_lock.new_broadcast_recv();

        {
            let a = state_lock.entities.new_component(ServerEntityState {
                name: Some("A".to_string()),
                mutable: Default::default(),
            });

            let b = state_lock.entities.new_component(ServerEntityState {
                name: Some("B".to_string()),
                mutable: ServerEntityStateUpdatable {
                    parent: Some(a),
                    ..Default::default()
                },
            });

            let _c = state_lock.entities.new_component(ServerEntityState {
                name: Some("C".to_string()),
                mutable: ServerEntityStateUpdatable {
                    parent: Some(b),
                    ..Default::default()
                },
            });
        }

        while let Ok(_msg) = recv.try_recv() {}
    }

    #[test]
    fn check_lookup_inspect() {
        let state = ServerState::new();

        let mut state_lock = state.lock().unwrap();

        let mut recv = state_lock.new_broadcast_recv();

        let a = state_lock.entities.new_component(ServerEntityState {
            name: Some("A".to_string()),
            mutable: ServerEntityStateUpdatable {
                tags: Some(vec!["item_a".to_string()]),
                ..Default::default()
            },
        });

        state_lock.entities.inspect(a.id(), |e| {
            assert_eq!(e.name.as_ref().unwrap(), "A");
            assert_eq!(e.mutable.tags, Some(vec!["item_a".to_string()]));
        });

        while let Ok(_msg) = recv.try_recv() {}
    }
}
