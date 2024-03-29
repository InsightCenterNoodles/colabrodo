//!
//! Client State
//!
//!
//! Some design notes: initial implementation was super async heavy, which was
//! very interesting, and incredibly convoluted. Having things in async meant
//! having to deal with the possible thread changes, which meant more guards,
//! etc. So here, we are going to avoid that.

use std::{collections::HashMap, fmt::Debug};

use ciborium::value::Value;
use colabrodo_common::{
    client_communication::{InvokeIDType, MethodInvokeMessage},
    components::LightStateUpdatable,
    nooid::*,
    server_communication::{MessageMethodReply, MethodException},
};

use crate::{
    client::{ClientChannels, InvokeContext, OutgoingMessage},
    components::*,
    delegate::*,
};

/// Type representing a dynamic method delegate
pub type MethodDelegate =
    dyn Delegate<IDType = MethodID, InitStateType = ClientMethodState>;

/// Type representing a dynamic signal delegate
pub type SignalDelegate =
    dyn Delegate<IDType = SignalID, InitStateType = ClientSignalState>;

/// Type representing a dynamic buffer delegate
pub type BufferDelegate =
    dyn Delegate<IDType = BufferID, InitStateType = BufferState>;

/// Type representing a dynamic buffer view delegate
pub type BufferViewDelegate =
    dyn Delegate<IDType = BufferViewID, InitStateType = ClientBufferViewState>;

/// Type representing a dynamic sampler delegate
pub type SamplerDelegate =
    dyn Delegate<IDType = SamplerID, InitStateType = SamplerState>;

/// Type representing a dynamic image delegate
pub type ImageDelegate =
    dyn Delegate<IDType = ImageID, InitStateType = ClientImageState>;

/// Type representing a dynamic texture delegate
pub type TextureDelegate =
    dyn Delegate<IDType = TextureID, InitStateType = ClientTextureState>;

/// Type representing a dynamic material delegate
pub type MaterialDelegate = dyn UpdatableDelegate<
    IDType = MaterialID,
    InitStateType = ClientMaterialState,
    UpdateStateType = ClientMaterialUpdate,
>;

/// Type representing a dynamic geometry delegate
pub type GeometryDelegate =
    dyn Delegate<IDType = GeometryID, InitStateType = ClientGeometryState>;

/// Type representing a dynamic light delegate
pub type LightDelegate = dyn UpdatableDelegate<
    IDType = LightID,
    InitStateType = LightState,
    UpdateStateType = LightStateUpdatable,
>;

/// Type representing a dynamic table delegate
pub type TableDelegate = dyn UpdatableDelegate<
    IDType = TableID,
    InitStateType = ClientTableState,
    UpdateStateType = ClientTableUpdate,
>;

/// Type representing a dynamic plot delegate
pub type PlotDelegate = dyn UpdatableDelegate<
    IDType = PlotID,
    InitStateType = ClientPlotState,
    UpdateStateType = ClientPlotUpdate,
>;

/// Type representing a dynamic entity delegate
pub type EntityDelegate = dyn UpdatableDelegate<
    IDType = EntityID,
    InitStateType = ClientEntityState,
    UpdateStateType = ClientEntityUpdate,
>;

/// Contains all the delegates for the client
pub struct ClientDelegateLists {
    pub method_list: ComponentList<MethodID, MethodDelegate>,
    pub signal_list: ComponentList<SignalID, SignalDelegate>,

    pub buffer_list: ComponentList<BufferID, BufferDelegate>,
    pub buffer_view_list: ComponentList<BufferViewID, BufferViewDelegate>,

    pub sampler_list: ComponentList<SamplerID, SamplerDelegate>,
    pub image_list: ComponentList<ImageID, ImageDelegate>,
    pub texture_list: ComponentList<TextureID, TextureDelegate>,

    pub material_list: ComponentList<MaterialID, MaterialDelegate>,
    pub geometry_list: ComponentList<GeometryID, GeometryDelegate>,

    pub light_list: ComponentList<LightID, LightDelegate>,

    pub table_list: ComponentList<TableID, TableDelegate>,
    pub plot_list: ComponentList<PlotID, PlotDelegate>,
    pub entity_list: ComponentList<EntityID, EntityDelegate>,

    pub document: Option<Box<dyn DocumentDelegate + Send>>,
}

impl ClientDelegateLists {
    /// Create a new client state, using a [DelegateMaker].
    pub fn new<Maker>(maker: &mut Maker) -> Self
    where
        Maker: DelegateMaker + Send,
    {
        Self {
            method_list: Default::default(),
            signal_list: Default::default(),
            buffer_list: Default::default(),
            buffer_view_list: Default::default(),
            sampler_list: Default::default(),
            image_list: Default::default(),
            texture_list: Default::default(),
            material_list: Default::default(),
            geometry_list: Default::default(),
            light_list: Default::default(),
            table_list: Default::default(),
            plot_list: Default::default(),
            entity_list: Default::default(),
            document: Some(maker.make_document()),
        }
    }

    pub(crate) fn clear(&mut self, maker: &mut dyn DelegateMaker) {
        self.method_list.clear();
        self.signal_list.clear();
        self.buffer_list.clear();
        self.buffer_view_list.clear();
        self.sampler_list.clear();
        self.image_list.clear();
        self.texture_list.clear();
        self.material_list.clear();
        self.geometry_list.clear();
        self.light_list.clear();
        self.table_list.clear();
        self.plot_list.clear();
        self.entity_list.clear();
        self.document = Some(maker.make_document());
    }
}

/// Contains all the state for a current client session
pub struct ClientState {
    pub output: tokio::sync::mpsc::UnboundedSender<OutgoingMessage>,

    pub maker: Box<dyn DelegateMaker>,

    pub delegate_lists: ClientDelegateLists,

    pub method_subs: HashMap<uuid::Uuid, InvokeContext>,
}

/// This trait is used to inform a client which delegate to construct when a new component is requested.
///
/// Define a struct that implements this trait and override a 'make_*' method that returns a type that satisfies the delegate trait.
pub trait DelegateMaker: Send {
    #[allow(unused_variables)]
    fn make_method(
        &mut self,
        id: MethodID,
        state: ClientMethodState,
        client: &mut ClientDelegateLists,
    ) -> Box<MethodDelegate> {
        Box::new(DefaultMethodDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_signal(
        &mut self,
        id: SignalID,
        state: ClientSignalState,
        client: &mut ClientDelegateLists,
    ) -> Box<SignalDelegate> {
        Box::new(DefaultSignalDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_buffer(
        &mut self,
        id: BufferID,
        state: BufferState,
        client: &mut ClientDelegateLists,
    ) -> Box<BufferDelegate> {
        Box::new(DefaultBufferDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_buffer_view(
        &mut self,
        id: BufferViewID,
        state: ClientBufferViewState,
        client: &mut ClientDelegateLists,
    ) -> Box<BufferViewDelegate> {
        Box::new(DefaultBufferViewDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_sampler(
        &mut self,
        id: SamplerID,
        state: SamplerState,
        client: &mut ClientDelegateLists,
    ) -> Box<SamplerDelegate> {
        Box::new(DefaultSamplerDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_image(
        &mut self,
        id: ImageID,
        state: ClientImageState,
        client: &mut ClientDelegateLists,
    ) -> Box<ImageDelegate> {
        Box::new(DefaultImageDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_texture(
        &mut self,
        id: TextureID,
        state: ClientTextureState,
        client: &mut ClientDelegateLists,
    ) -> Box<TextureDelegate> {
        Box::new(DefaultTextureDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_material(
        &mut self,
        id: MaterialID,
        state: ClientMaterialState,
        client: &mut ClientDelegateLists,
    ) -> Box<MaterialDelegate> {
        Box::new(DefaultMaterialDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_geometry(
        &mut self,
        id: GeometryID,
        state: ClientGeometryState,
        client: &mut ClientDelegateLists,
    ) -> Box<GeometryDelegate> {
        Box::new(DefaultGeometryDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_light(
        &mut self,
        id: LightID,
        state: LightState,
        client: &mut ClientDelegateLists,
    ) -> Box<LightDelegate> {
        Box::new(DefaultLightDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_table(
        &mut self,
        id: TableID,
        state: ClientTableState,
        client: &mut ClientDelegateLists,
    ) -> Box<TableDelegate> {
        Box::new(DefaultTableDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_plot(
        &mut self,
        id: PlotID,
        state: ClientPlotState,
        client: &mut ClientDelegateLists,
    ) -> Box<PlotDelegate> {
        Box::new(DefaultPlotDelegate::new(state))
    }

    #[allow(unused_variables)]
    fn make_entity(
        &mut self,
        id: EntityID,
        state: ClientEntityState,
        client: &mut ClientDelegateLists,
    ) -> Box<EntityDelegate> {
        Box::new(DefaultEntityDelegate::new(state))
    }

    fn make_document(&mut self) -> Box<dyn DocumentDelegate + Send> {
        Box::<DefaultDocumentDelegate>::default()
    }
}

impl Debug for ClientState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientState").finish()
    }
}

impl ClientState {
    /// Create a new client state, providing communication channels, and a delegate maker.
    pub fn new<Maker>(channels: &ClientChannels, mut maker: Maker) -> Self
    where
        Maker: DelegateMaker + 'static,
    {
        let delegate_lists = ClientDelegateLists::new(&mut maker);

        Self {
            output: channels.from_client_tx.clone(),

            maker: Box::new(maker),

            delegate_lists,

            method_subs: Default::default(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.delegate_lists.clear(&mut *self.maker);
        self.method_subs.clear();
    }

    /// Invoke a method on a component.
    ///
    /// # Parameters
    /// - state: The client state to invoke on
    /// - method_id: The method id to invoke
    /// - context: Component to invoke the method on
    /// - args: A list of CBOR arguments to send to the server
    ///
    /// # Return
    /// The unique identifier of the invocation
    pub fn invoke_method(
        &mut self,
        method_id: MethodID,
        context: InvokeContext,
        args: Vec<Value>,
    ) -> uuid::Uuid {
        let invoke_id = uuid::Uuid::new_v4();

        let content = OutgoingMessage::MethodInvoke(MethodInvokeMessage {
            method: method_id,
            context: match context {
                InvokeContext::Document => None,
                InvokeContext::Entity(id) => Some(InvokeIDType::Entity(id)),
                InvokeContext::Table(id) => Some(InvokeIDType::Table(id)),
                InvokeContext::Plot(id) => Some(InvokeIDType::Plot(id)),
            },
            invoke_id: Some(invoke_id.to_string()),
            args,
        });

        {
            // careful not to hold a lock across an await...

            self.method_subs.insert(invoke_id, context);

            self.output
                .send(content)
                .map_err(|_| {
                    MethodException::internal_error(Some(
                        "Unable to send method invocation.",
                    ))
                })
                .unwrap();
        }

        invoke_id
    }

    pub(crate) fn handle_signal(&mut self, invoke: ClientMessageSignalInvoke) {
        let sig_id = SignalID(invoke.id);
        if let Some(id) = invoke.context {
            if let Some(entity) = id.entity {
                let del = self.delegate_lists.entity_list.take(&entity);

                if let Some(mut del) = del {
                    del.on_signal(
                        sig_id,
                        &mut self.delegate_lists,
                        invoke.signal_data,
                    );

                    self.delegate_lists.entity_list.replace(&entity, del);
                }
            } else if let Some(plot) = id.plot {
                let del = self.delegate_lists.plot_list.take(&plot);

                if let Some(mut del) = del {
                    del.on_signal(
                        sig_id,
                        &mut self.delegate_lists,
                        invoke.signal_data,
                    );

                    self.delegate_lists.plot_list.replace(&plot, del);
                }
            } else if let Some(table) = id.table {
                let del = self.delegate_lists.table_list.take(&table);

                if let Some(mut del) = del {
                    del.on_signal(
                        sig_id,
                        &mut self.delegate_lists,
                        invoke.signal_data,
                    );

                    self.delegate_lists.table_list.replace(&table, del);
                }
            }
        } else {
            let del = std::mem::take(&mut self.delegate_lists.document);

            if let Some(mut del) = del {
                del.on_signal(sig_id, self, invoke.signal_data);
                self.delegate_lists.document = Some(del);
            }
        }
    }

    pub(crate) fn handle_method_reply(
        &mut self,
        id: uuid::Uuid,
        msg: MessageMethodReply,
    ) {
        if let Some(callback) = self.method_subs.remove(&id) {
            //callback(self, msg);
            match callback {
                InvokeContext::Document => {
                    if let Some(mut del) = self.delegate_lists.document.take() {
                        del.on_method_reply(self, id, msg);
                        self.delegate_lists.document = Some(del);
                    }
                }
                InvokeContext::Entity(del_id) => {
                    // we have to do this dance, because we cant take a mut ref,
                    // to self (through getting the delegate) and THEN pass
                    // it to the delegate for editing.
                    let del = self.delegate_lists.entity_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(&mut self.delegate_lists, id, msg);
                        self.delegate_lists.entity_list.replace(&del_id, del);
                    }
                }
                InvokeContext::Table(del_id) => {
                    let del = self.delegate_lists.table_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(&mut self.delegate_lists, id, msg);
                        self.delegate_lists.table_list.replace(&del_id, del);
                    }
                }
                InvokeContext::Plot(del_id) => {
                    let del = self.delegate_lists.plot_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(&mut self.delegate_lists, id, msg);
                        self.delegate_lists.plot_list.replace(&del_id, del);
                    }
                }
            };
        }
    }

    /// Issue a shutdown for the client and wait for all client machinery to stop.
    pub fn shutdown(&mut self) {
        self.output.send(OutgoingMessage::Close).unwrap();
    }
}
