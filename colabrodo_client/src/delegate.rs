use std::{any::Any, collections::HashMap, fmt::Debug, hash::Hash};

use colabrodo_common::{
    components::LightStateUpdatable, nooid::*,
    server_communication::MessageMethodReply,
};

use crate::{client_state::*, components::*};

/// Basic trait for a delegate.
pub trait Delegate: Send {
    type IDType;
    type InitStateType;

    /// Called when the delegate is about to be destroyed
    fn on_delete(&mut self) {}

    /// As we use virtual types, this should be implemented, likely with
    /// `fn as_any(&self) -> &dyn Any { self }`
    fn as_any(&self) -> &dyn Any;

    /// As we use virtual types, this should be implemented, likely with
    /// `fn as_any_mut(&self) -> &mut dyn Any { self }`
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A trait for delegates that can be updated.
pub trait UpdatableDelegate: Delegate {
    type UpdateStateType;

    /// Called when the server wishes to update the delegate
    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
    }

    /// Called when the server has issued a signal to this delegate
    #[allow(unused_variables)]
    fn on_signal(
        &mut self,
        id: SignalID,
        client: &mut ClientDelegateLists,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    /// Called when the server has a reply from a method invoked on this delegate
    #[allow(unused_variables)]
    fn on_method_reply(
        &mut self,
        client: &mut ClientDelegateLists,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }
}

/// Trait for the delegate that handles the document
pub trait DocumentDelegate {
    /// Called when the server has issued a signal to this delegate
    #[allow(unused_variables)]
    fn on_signal(
        &mut self,
        id: SignalID,
        client: &mut ClientState,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    /// Called when the server has a reply from a method invoked on this delegate
    #[allow(unused_variables)]
    fn on_method_reply(
        &mut self,
        client: &mut ClientState,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }

    /// Called when the document should be updated
    #[allow(unused_variables)]
    fn on_document_update(
        &mut self,
        client: &mut ClientState,
        update: ClientDocumentUpdate,
    ) {
    }

    /// Called when the document is ready
    #[allow(unused_variables)]
    fn on_ready(&mut self, client: &mut ClientState) {}
}

// =============================================================================

/// [ComponentList] contains a list of delegates
#[derive(Debug)]
pub struct ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: ?Sized + Send,
{
    name_map: HashMap<String, IDType>,
    name_rev_map: HashMap<IDType, String>,
    components: HashMap<IDType, Box<Del>>,
}

impl<IDType, Del> Default for ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: ?Sized + Send,
{
    fn default() -> Self {
        Self {
            name_map: HashMap::new(),
            name_rev_map: HashMap::new(),
            components: HashMap::new(),
        }
    }
}

impl<IDType, Del> ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: Delegate + ?Sized + Send,
{
    /// Called when a delegate should be created
    pub(crate) fn on_create(
        &mut self,
        id: IDType,
        name: Option<String>,
        del: Box<Del>,
    ) {
        self.components.insert(id, del);

        if let Some(n) = name {
            self.name_map.insert(n.clone(), id);
            self.name_rev_map.insert(id, n);
        }
    }

    /// Called when a delegate should be destroyed
    pub(crate) fn on_delete(&mut self, id: IDType) {
        let mut pack = self.components.remove(&id).unwrap();
        pack.on_delete();
        let name = self.name_rev_map.remove(&id);
        if let Some(n) = name {
            self.name_map.remove(&n);
        }
    }

    /// Find a component state by ID
    pub fn find(&self, id: &IDType) -> Option<&Del> {
        self.components.get(id).as_ref().map(|x| x as _)
    }

    /// Find a component state by ID (mutable)
    pub fn find_mut(&mut self, id: &IDType) -> Option<&mut Box<Del>> {
        self.components.get_mut(id)
    }

    /// This is a HACK. We can't take a mut reference. See [ClientState::handle_method_reply]
    pub(crate) fn take(&mut self, id: &IDType) -> Option<Box<Del>> {
        self.components.remove(id)
    }

    /// This is a HACK
    pub(crate) fn replace(&mut self, id: &IDType, del: Box<Del>) {
        self.components.insert(*id, del);
    }

    /// Find a component ID by a name
    pub fn get_id_by_name(&self, name: &str) -> Option<IDType> {
        self.name_map.get(name).cloned()
    }

    /// Get a component state by a name
    pub fn get_state_by_name(&self, name: &str) -> Option<&Del> {
        self.find(self.name_map.get(name)?)
    }

    pub fn clear(&mut self) {
        self.name_map.clear();
        self.components.clear();
    }

    /// Get a delegate by ID, downcasting to the provided delegate type
    pub fn find_as<T: 'static>(&self, id: &IDType) -> Option<&T> {
        self.find(id).and_then(|f| f.as_any().downcast_ref::<T>())
    }

    /// Get a mutable delegate by ID, downcasting to the provided delegate type
    pub fn find_as_mut<T: 'static>(&mut self, id: &IDType) -> Option<&mut T> {
        self.find_mut(id)
            .and_then(|f| f.as_any_mut().downcast_mut::<T>())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&IDType, &Box<Del>)> {
        self.components.iter()
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&IDType, &mut Box<Del>)> {
        self.components.iter_mut()
    }
}

// =============================================================================

macro_rules! define_default_delegates {
    ($id:ident, $idtype:ty, $init:ty) => {
        pub struct $id {
            _state: $init,
        }

        impl $id {
            pub fn new(state: $init) -> Self {
                Self { _state: state }
            }
        }

        impl Delegate for $id {
            type IDType = $idtype;
            type InitStateType = $init;

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
    };
}

define_default_delegates!(DefaultMethodDelegate, MethodID, ClientMethodState);
define_default_delegates!(DefaultSignalDelegate, SignalID, ClientSignalState);
define_default_delegates!(DefaultBufferDelegate, BufferID, BufferState);
define_default_delegates!(
    DefaultBufferViewDelegate,
    BufferViewID,
    ClientBufferViewState
);
define_default_delegates!(DefaultSamplerDelegate, SamplerID, SamplerState);
define_default_delegates!(DefaultImageDelegate, ImageID, ClientImageState);
define_default_delegates!(
    DefaultTextureDelegate,
    TextureID,
    ClientTextureState
);
define_default_delegates!(
    DefaultMaterialDelegate,
    MaterialID,
    ClientMaterialState
);
define_default_delegates!(
    DefaultGeometryDelegate,
    GeometryID,
    ClientGeometryState
);
define_default_delegates!(DefaultLightDelegate, LightID, LightState);
define_default_delegates!(DefaultTableDelegate, TableID, ClientTableState);
define_default_delegates!(DefaultPlotDelegate, PlotID, ClientPlotState);
define_default_delegates!(DefaultEntityDelegate, EntityID, ClientEntityState);

impl UpdatableDelegate for DefaultMaterialDelegate {
    type UpdateStateType = ClientMaterialUpdate;
}

impl UpdatableDelegate for DefaultLightDelegate {
    type UpdateStateType = LightStateUpdatable;
}

impl UpdatableDelegate for DefaultTableDelegate {
    type UpdateStateType = ClientTableUpdate;
}

impl UpdatableDelegate for DefaultPlotDelegate {
    type UpdateStateType = ClientPlotUpdate;
}

impl UpdatableDelegate for DefaultEntityDelegate {
    type UpdateStateType = ClientEntityUpdate;
}

#[derive(Default)]
pub struct DefaultDocumentDelegate {}

impl DocumentDelegate for DefaultDocumentDelegate {}
