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

pub trait DelegateProvider {
    type MethodDelegate: Delegate<
        IDType = MethodID,
        InitStateType = ClientMethodState,
    >;
    type SignalDelegate: Delegate<
        IDType = SignalID,
        InitStateType = ClientSignalState,
    >;
    type BufferDelegate: Delegate<
        IDType = BufferID,
        InitStateType = BufferState,
    >;
    type BufferViewDelegate: Delegate<
        IDType = BufferViewID,
        InitStateType = ClientBufferViewState,
    >;
    type SamplerDelegate: Delegate<
        IDType = SamplerID,
        InitStateType = SamplerState,
    >;
    type ImageDelegate: Delegate<
        IDType = ImageID,
        InitStateType = ClientImageState,
    >;
    type TextureDelegate: Delegate<
        IDType = TextureID,
        InitStateType = ClientTextureState,
    >;
    type MaterialDelegate: UpdatableDelegate<
        IDType = MaterialID,
        InitStateType = ClientMaterialState,
        UpdateStateType = ClientMaterialUpdate,
    >;
    type GeometryDelegate: Delegate<
        IDType = GeometryID,
        InitStateType = ClientGeometryState,
    >;
    type LightDelegate: UpdatableDelegate<
        IDType = LightID,
        InitStateType = LightState,
        UpdateStateType = LightStateUpdatable,
    >;
    type TableDelegate: UpdatableDelegate<
        IDType = TableID,
        InitStateType = ClientTableState,
        UpdateStateType = ClientTableUpdate,
    >;
    type PlotDelegate: UpdatableDelegate<
        IDType = PlotID,
        InitStateType = ClientPlotState,
        UpdateStateType = ClientPlotUpdate,
    >;
    type EntityDelegate: UpdatableDelegate<
        IDType = EntityID,
        InitStateType = ClientEntityState,
        UpdateStateType = ClientEntityUpdate,
    >;

    type DocumentDelegate: DocumentDelegate + Default;
}

pub struct ClientState<Provider: DelegateProvider> {
    pub output: tokio::sync::mpsc::UnboundedSender<OutgoingMessage>,

    pub method_list: ComponentList<MethodID, Provider::MethodDelegate>,
    pub signal_list: ComponentList<SignalID, Provider::SignalDelegate>,

    pub buffer_list: ComponentList<BufferID, Provider::BufferDelegate>,
    pub buffer_view_list:
        ComponentList<BufferViewID, Provider::BufferViewDelegate>,

    pub sampler_list: ComponentList<SamplerID, Provider::SamplerDelegate>,
    pub image_list: ComponentList<ImageID, Provider::ImageDelegate>,
    pub texture_list: ComponentList<TextureID, Provider::TextureDelegate>,

    pub material_list: ComponentList<MaterialID, Provider::MaterialDelegate>,
    pub geometry_list: ComponentList<GeometryID, Provider::GeometryDelegate>,

    pub light_list: ComponentList<LightID, Provider::LightDelegate>,

    pub table_list: ComponentList<TableID, Provider::TableDelegate>,
    pub plot_list: ComponentList<PlotID, Provider::PlotDelegate>,
    pub entity_list: ComponentList<EntityID, Provider::EntityDelegate>,

    pub document: Option<Box<Provider::DocumentDelegate>>,

    pub method_subs: HashMap<uuid::Uuid, InvokeContext>,
}

impl<Provider> Debug for ClientState<Provider>
where
    Provider: DelegateProvider,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientState").finish()
    }
}

impl<Provider: DelegateProvider> ClientState<Provider> {
    /// Create a new client state, using previously created channels.
    pub fn new(channels: &ClientChannels) -> Self {
        Self {
            output: channels.from_client_tx.clone(),

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
            document: Some(Box::new(Provider::DocumentDelegate::default())),
            method_subs: Default::default(),
        }
    }

    pub(crate) fn clear(&mut self) {
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
        self.document = Default::default();
        self.method_subs.clear();
    }

    // /// Update document method/signal handlers
    // pub(crate) fn update_method_signals(&mut self) {
    //     if let Some(siglist) = &self.document_communication.signals_list {
    //         let siglist: Vec<_> = siglist
    //             .iter()
    //             .filter(|x| !self.signal_subs.contains_key(x))
    //             .collect();

    //         for sid in siglist {
    //             self.signal_subs.remove(sid);
    //         }
    //     }
    // }

    // /// Subscribe to a signal on the document.
    // ///
    // /// # Returns
    // ///
    // /// Returns [None] if the subscription fails (non-existing component, etc). Otherwise returns a channel to be subscribed to.
    // pub fn subscribe_signal(&mut self, signal: SignalID) -> Option<()> {
    //     // if let Some(list) = &mut self.document_communication.signals_list {
    //     //     log::debug!("Searching for signal {signal:?} in {list:?}");
    //     //     if list.iter().any(|&f| f == signal) {
    //     //         return Some(
    //     //             self.signal_subs
    //     //                 .entry(signal)
    //     //                 .or_insert_with(|| {
    //     //                     tokio::sync::broadcast::channel(16).0
    //     //                 })
    //     //                 .subscribe(),
    //     //         );
    //     //     }
    //     // }
    //     // log::debug!("Unable to find requested signal: {signal:?}");
    //     None
    // }

    // /// Unsubscribe to a document signal
    // pub fn unsubscribe_signal(&mut self, signal: SignalID) {
    //     self.signal_subs.remove(&signal);
    // }

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
                let del = self.entity_list.take(&entity);

                if let Some(mut del) = del {
                    del.on_signal(sig_id, self, invoke.signal_data);

                    self.entity_list.replace(&entity, del);
                }
            } else if let Some(plot) = id.plot {
                let del = self.plot_list.take(&plot);

                if let Some(mut del) = del {
                    del.on_signal(sig_id, self, invoke.signal_data);

                    self.plot_list.replace(&plot, del);
                }
            } else if let Some(table) = id.table {
                let del = self.table_list.take(&table);

                if let Some(mut del) = del {
                    del.on_signal(sig_id, self, invoke.signal_data);

                    self.table_list.replace(&table, del);
                }
            }
        } else {
            let del = std::mem::take(&mut self.document);

            if let Some(mut del) = del {
                del.on_signal(sig_id, self, invoke.signal_data);
                self.document = Some(del);
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
                    if let Some(mut del) = self.document.take() {
                        del.on_method_reply(self, id, msg);
                        self.document = Some(del);
                    }
                }
                InvokeContext::Entity(del_id) => {
                    // we have to do this dance, because we cant take a mut ref,
                    // to self (through getting the delegate) and THEN pass
                    // it to the delegate for editing.
                    let del = self.entity_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(self, id, msg);
                        self.entity_list.replace(&del_id, del);
                    }
                }
                InvokeContext::Table(del_id) => {
                    let del = self.table_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(self, id, msg);
                        self.table_list.replace(&del_id, del);
                    }
                }
                InvokeContext::Plot(del_id) => {
                    let del = self.plot_list.take(&del_id);

                    if let Some(mut del) = del {
                        del.on_method_reply(self, id, msg);
                        self.plot_list.replace(&del_id, del);
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
