use std::{any::Any, collections::HashMap, fmt::Debug, hash::Hash};

use colabrodo_common::{
    components::LightStateUpdatable, nooid::*,
    server_communication::MessageMethodReply,
};

use crate::{client_state::*, components::*};

pub trait Delegate {
    type IDType;
    type InitStateType;

    fn on_delete(&mut self) {}

    fn as_any(&self) -> &dyn Any;
}

pub trait UpdatableDelegate: Delegate {
    type UpdateStateType;

    #[allow(unused_variables)]
    fn on_update(&mut self, state: Self::UpdateStateType) {}

    #[allow(unused_variables)]
    fn on_signal(
        &mut self,
        id: SignalID,
        client: &mut ClientState,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    #[allow(unused_variables)]
    fn on_method_reply(
        &mut self,
        client: &mut ClientState,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }
}

pub trait DocumentDelegate {
    #[allow(unused_variables)]
    fn on_signal(
        &mut self,
        id: SignalID,
        client: &mut ClientState,
        args: Vec<ciborium::value::Value>,
    ) {
    }

    #[allow(unused_variables)]
    fn on_method_reply(
        &mut self,
        client: &mut ClientState,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
    }

    #[allow(unused_variables)]
    fn on_document_update(
        &mut self,
        client: &mut ClientState,
        update: ClientDocumentUpdate,
    ) {
    }

    #[allow(unused_variables)]
    fn on_ready(&mut self, client: &mut ClientState) {}
}

// =============================================================================

#[derive(Debug)]
pub struct ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: ?Sized,
{
    name_map: HashMap<String, IDType>,
    name_rev_map: HashMap<IDType, String>,
    components: HashMap<IDType, Box<Del>>,
}

impl<IDType, Del> Default for ComponentList<IDType, Del>
where
    IDType: Eq + Hash + Copy,
    Del: ?Sized,
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
    Del: Delegate + ?Sized,
{
    pub fn on_create(
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

    pub fn on_delete(&mut self, id: IDType) {
        let mut pack = self.components.remove(&id).unwrap();
        pack.on_delete();
        let name = self.name_rev_map.remove(&id);
        if let Some(n) = name {
            self.name_map.remove(&n);
        }
    }

    /// Find a component state by ID
    pub fn find(&self, id: &IDType) -> Option<&Box<Del>> {
        self.components.get(id)
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
    pub fn get_state_by_name(&self, name: &str) -> Option<&Box<Del>> {
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
    Del: UpdatableDelegate + ?Sized,
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

            // fn on_new(
            //     _id: Self::IDType,
            //     state: Self::InitStateType,
            //     _client: &mut ClientState,
            // ) -> Self {
            //     Self { _state: state }
            // }
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
