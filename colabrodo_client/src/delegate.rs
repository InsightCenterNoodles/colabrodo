use std::{collections::HashMap, fmt::Debug, hash::Hash};

use colabrodo_common::{
    components::LightStateUpdatable, nooid::*,
    server_communication::MessageMethodReply,
};

use crate::{client_state::*, components::*};

pub trait Delegate {
    type IDType;
    type InitStateType;

    fn on_new<Provider: DelegateProvider>(
        id: Self::IDType,
        state: Self::InitStateType,
        client: &mut ClientState<Provider>,
    ) -> Self;
    fn on_delete(&mut self) {}
}

pub trait UpdatableDelegate: Delegate {
    type UpdateStateType;

    #[allow(unused_variables)]
    fn on_update(&mut self, state: Self::UpdateStateType) {}

    #[allow(unused_variables)]
    fn on_signal<Provider: DelegateProvider>(
        &mut self,
        id: SignalID,
        client: &mut ClientState<Provider>,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    #[allow(unused_variables)]
    fn on_method_reply<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }
}

pub trait DocumentDelegate {
    #[allow(unused_variables)]
    fn on_signal<Provider: DelegateProvider>(
        &mut self,
        id: SignalID,
        client: &mut ClientState<Provider>,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    #[allow(unused_variables)]
    fn on_method_reply<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }

    #[allow(unused_variables)]
    fn on_document_update<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
        update: ClientDocumentUpdate,
    ) {
    }

    #[allow(unused_variables)]
    fn on_ready<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
    ) {
    }
}

// =============================================================================

#[derive(Debug)]
pub struct ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: Delegate,
{
    name_map: HashMap<String, IDType>,
    name_rev_map: HashMap<IDType, String>,
    components: HashMap<IDType, Del>,
}

impl<IDType, Del> Default for ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: Delegate,
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
    Del: Delegate,
{
    pub fn on_create(&mut self, id: IDType, name: Option<String>, del: Del) {
        self.components.insert(id, del);

        if let Some(n) = name {
            self.name_map.insert(n.clone(), id);
            self.name_rev_map.insert(id, n);
        }
    }

    pub fn on_delete(&mut self, id: IDType) {
        let mut pack = self.components.remove(&id).unwrap();
        pack.on_delete();
        let name = self.name_rev_map.remove(&id);
        if let Some(n) = name {
            self.name_map.remove(&n);
        }
    }

    /// Find a component state by ID
    pub fn find(&self, id: &IDType) -> Option<&Del> {
        self.components.get(id)
    }

    /// Find a component state by ID (mutable)
    pub fn find_mut(&mut self, id: &IDType) -> Option<&mut Del> {
        self.components.get_mut(id)
    }

    /// This is a HACK. We can't take a mut reference. See [ClientState::handle_method_reply]
    pub(crate) fn take(&mut self, id: &IDType) -> Option<Del> {
        self.components.remove(id)
    }

    /// This is a HACK
    pub(crate) fn replace(&mut self, id: &IDType, del: Del) {
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
}

impl<IDType, Del> ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: UpdatableDelegate,
{
    pub fn on_update(&mut self, id: IDType, update: Del::UpdateStateType) {
        if let Some(item) = self.components.get_mut(&id) {
            item.on_update(update);
        }
    }
}

// =============================================================================

macro_rules! define_default_delegates {
    ($id:ident, $idtype:ty, $init:ty) => {
        pub struct $id {
            _state: $init,
        }

        impl Delegate for $id {
            type IDType = $idtype;
            type InitStateType = $init;

            fn on_new<Provider: DelegateProvider>(
                _id: Self::IDType,
                state: Self::InitStateType,
                _client: &mut ClientState<Provider>,
            ) -> Self {
                Self { _state: state }
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
