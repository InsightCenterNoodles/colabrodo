use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_client::components::*;
use colabrodo_client::delegate::*;
use colabrodo_common::components::LightStateUpdatable;
use colabrodo_common::nooid::*;
use colabrodo_server::server_messages::*;
use std::sync::Weak;
use std::sync::{Arc, Mutex};

use crate::replay_state::ReplayerServerState;

pub struct Maker {
    server_link: Weak<Mutex<ReplayerServerState>>,
}

macro_rules! make_a_delegate {
    ($self:ident, $id:ident, $loc:ident, $del:ty, $converted:expr) => {{
        let app_ptr = $self.server_link.upgrade().unwrap();
        let app_lock = app_ptr.lock().unwrap();

        let server_ptr = app_lock.server_state();
        let mut server_lock = server_ptr.lock().unwrap();

        let reference = server_lock.$loc.new_component($converted);

        Box::new(<$del>::new($id, reference))
    }};
}

impl DelegateMaker for Maker {
    fn make_method(
        &mut self,
        id: MethodID,
        state: ClientMethodState,
        _client: &mut ClientDelegateLists,
    ) -> Box<MethodDelegate> {
        make_a_delegate!(
            self,
            id,
            methods,
            RMethodDelegate,
            ServerMethodState {
                name: state.name,
                doc: state.doc,
                return_doc: state.return_doc,
                arg_doc: state.arg_doc,
                state: Default::default(),
            }
        )
    }

    fn make_signal(
        &mut self,
        id: SignalID,
        state: ClientSignalState,
        _client: &mut ClientDelegateLists,
    ) -> Box<SignalDelegate> {
        make_a_delegate!(
            self,
            id,
            signals,
            RSignalDelegate,
            ServerSignalState {
                name: state.name,
                doc: state.doc,
                arg_doc: state.arg_doc,
                state: Default::default(),
            }
        )
    }

    fn make_buffer(
        &mut self,
        id: BufferID,
        state: BufferState,
        _client: &mut ClientDelegateLists,
    ) -> Box<BufferDelegate> {
        make_a_delegate!(
            self,
            id,
            buffers,
            RBufferDelegate,
            BufferState {
                name: state.name,
                size: state.size,
                inline_bytes: state.inline_bytes,
                uri_bytes: state.uri_bytes
            }
        )
    }

    fn make_buffer_view(
        &mut self,
        id: BufferViewID,
        state: ClientBufferViewState,
        client: &mut ClientDelegateLists,
    ) -> Box<BufferViewDelegate> {
        let buff = client.buffer_list.find(&state.source_buffer).unwrap();
        let buff = buff.as_any().downcast_ref::<RBufferDelegate>().unwrap();

        make_a_delegate!(
            self,
            id,
            buffer_views,
            RBufferViewDelegate,
            ServerBufferViewState {
                name: state.name,
                source_buffer: buff.reference.clone(),
                view_type: state.view_type,
                offset: state.offset,
                length: state.length,
            }
        )
    }

    fn make_sampler(
        &mut self,
        id: SamplerID,
        state: SamplerState,
        _client: &mut ClientDelegateLists,
    ) -> Box<SamplerDelegate> {
        make_a_delegate!(self, id, samplers, RSamplerDelegate, state)
    }

    fn make_image(
        &mut self,
        id: ImageID,
        state: ClientImageState,
        client: &mut ClientDelegateLists,
    ) -> Box<ImageDelegate> {
        let buff = state
            .source
            .buffer_source
            .and_then(|id| client.buffer_view_list.find(&id))
            .and_then(|f| f.as_any().downcast_ref::<RBufferViewDelegate>())
            .map(|f| f.reference.clone());

        let a = ServerImageState {
            name: state.name,
            source: ServerImageStateSource {
                buffer_source: buff,
                uri_source: state.source.uri_source,
            },
        };

        make_a_delegate!(self, id, images, RImageDelegate, a)
    }

    fn make_texture(
        &mut self,
        id: TextureID,
        state: ClientTextureState,
        client: &mut ClientDelegateLists,
    ) -> Box<TextureDelegate> {
        let image = client
            .image_list
            .find_as::<RImageDelegate>(&state.image)
            .unwrap()
            .reference
            .clone();

        let sampler = state
            .sampler
            .and_then(|f| client.sampler_list.find_as::<RSamplerDelegate>(&f))
            .map(|f| f.reference.clone());

        make_a_delegate!(
            self,
            id,
            textures,
            RTextureDelegate,
            ServerTextureState {
                name: state.name,
                image,
                sampler,
            }
        )
    }

    fn make_material(
        &mut self,
        id: MaterialID,
        state: ClientMaterialState,
        client: &mut ClientDelegateLists,
    ) -> Box<MaterialDelegate> {
        make_a_delegate!(
            self,
            id,
            materials,
            RMaterialDelegate,
            ServerMaterialState {
                name: state.name,
                mutable: convert_material_update(client, state.mutable)
            }
        )
    }

    fn make_geometry(
        &mut self,
        id: GeometryID,
        state: ClientGeometryState,
        client: &mut ClientDelegateLists,
    ) -> Box<GeometryDelegate> {
        make_a_delegate!(
            self,
            id,
            geometries,
            RGeometryDelegate,
            ServerGeometryState {
                name: state.name,
                patches: state
                    .patches
                    .into_iter()
                    .map(|f| {
                        ServerGeometryPatch {
                            attributes: f
                                .attributes
                                .into_iter()
                                .map(|f| ServerGeometryAttribute {
                                    view: client
                                        .buffer_view_list
                                        .find_as::<RBufferViewDelegate>(&f.view)
                                        .unwrap()
                                        .reference
                                        .clone(),
                                    semantic: f.semantic,
                                    channel: f.channel,
                                    offset: f.offset,
                                    stride: f.stride,
                                    format: f.format,
                                    minimum_value: f.minimum_value,
                                    maximum_value: f.maximum_value,
                                    normalized: f.normalized,
                                })
                                .collect(),
                            vertex_count: f.vertex_count,
                            indices: f.indices.map(|f| ServerGeometryIndex {
                                view: client
                                    .buffer_view_list
                                    .find_as::<RBufferViewDelegate>(&f.view)
                                    .unwrap()
                                    .reference
                                    .clone(),
                                count: f.count,
                                offset: f.offset,
                                stride: f.stride,
                                format: f.format,
                            }),
                            patch_type: f.patch_type,
                            material: client
                                .material_list
                                .find_as::<RMaterialDelegate>(&f.material)
                                .unwrap()
                                .reference
                                .clone(),
                        }
                    })
                    .collect()
            }
        )
    }

    fn make_light(
        &mut self,
        id: LightID,
        state: LightState,
        client: &mut ClientDelegateLists,
    ) -> Box<LightDelegate> {
        make_a_delegate!(
            self,
            id,
            lights,
            RLightDelegate,
            ServerLightState {
                name: state.name,
                light_type: state.light_type,
                mutable: convert_light_update(client, state.mutable)
            }
        )
    }

    fn make_table(
        &mut self,
        id: TableID,
        state: ClientTableState,
        client: &mut ClientDelegateLists,
    ) -> Box<TableDelegate> {
        make_a_delegate!(
            self,
            id,
            tables,
            RTableDelegate,
            ServerTableState {
                name: state.name,
                mutable: convert_table_update(client, state.mutable)
            }
        )
    }

    fn make_plot(
        &mut self,
        id: PlotID,
        state: ClientPlotState,
        client: &mut ClientDelegateLists,
    ) -> Box<PlotDelegate> {
        make_a_delegate!(
            self,
            id,
            plots,
            RPlotDelegate,
            ServerPlotState {
                name: state.name,
                mutable: convert_plot_update(client, state.mutable),
            }
        )
    }

    fn make_entity(
        &mut self,
        id: EntityID,
        state: ClientEntityState,
        client: &mut ClientDelegateLists,
    ) -> Box<EntityDelegate> {
        make_a_delegate!(
            self,
            id,
            entities,
            REntityDelegate,
            ServerEntityState {
                name: state.name,
                mutable: convert_entity_update(client, state.mutable)
            }
        )
    }

    fn make_document(&mut self) -> Box<dyn DocumentDelegate + Send> {
        Box::new(ReplayDocDelegate::new(self.server_link.clone()))
    }
}

fn convert_representation(
    client: &ClientDelegateLists,
    rep: Option<ClientEntityRepresentation>,
) -> Option<ServerEntityRepresentation> {
    rep.map(|f| {
        let current = f.extract_current();
        match current {
            colabrodo_common::components::CurrentRepresentation::Render(
                render,
            ) => ServerEntityRepresentation::new_render(
                ServerRenderRepresentation {
                    mesh: client
                        .geometry_list
                        .find_as::<RGeometryDelegate>(&render.mesh)
                        .unwrap()
                        .reference
                        .clone(),
                    instances: render.instances.map(|inst| {
                        ServerGeometryInstance {
                            view: client
                                .buffer_view_list
                                .find_as::<RBufferViewDelegate>(&inst.view)
                                .unwrap()
                                .reference
                                .clone(),
                            stride: inst.stride,
                            bb: inst.bb,
                        }
                    }),
                },
            ),
            colabrodo_common::components::CurrentRepresentation::Null(_) => {
                ServerEntityRepresentation::new_null()
            }
            colabrodo_common::components::CurrentRepresentation::Text(x) => {
                ServerEntityRepresentation::new_text(x)
            }
            colabrodo_common::components::CurrentRepresentation::Web(x) => {
                ServerEntityRepresentation::new_web(x)
            }
        }
    })
}

macro_rules! gen_convert_list {
    ($fn_name:ident, $id:ty, $refty:ty, $cast:ty, $list:ident) => {
        fn $fn_name(
            client: &ClientDelegateLists,
            ids: Option<Vec<$id>>,
        ) -> Option<Vec<$refty>> {
            ids.map(|f| {
                f.into_iter()
                    .filter_map(|id| client.$list.find_as::<$cast>(&id))
                    .map(|g| g.reference.clone())
                    .collect()
            })
        }
    };
}

gen_convert_list!(
    convert_methods,
    MethodID,
    MethodReference,
    RMethodDelegate,
    method_list
);

gen_convert_list!(
    convert_signals,
    SignalID,
    SignalReference,
    RSignalDelegate,
    signal_list
);

gen_convert_list!(
    convert_lights,
    LightID,
    LightReference,
    RLightDelegate,
    light_list
);

gen_convert_list!(
    convert_plots,
    PlotID,
    PlotReference,
    RPlotDelegate,
    plot_list
);

gen_convert_list!(
    convert_tables,
    TableID,
    TableReference,
    RTableDelegate,
    table_list
);

fn convert_opt_texture_ref(
    client: &mut ClientDelegateLists,
    source: Option<ClientTextureRef>,
) -> Option<ServerTextureRef> {
    source.map(|f| convert_texture_ref(client, f))
}

fn convert_texture_ref(
    client: &mut ClientDelegateLists,
    source: ClientTextureRef,
) -> ServerTextureRef {
    let texture = client
        .texture_list
        .find_as::<RTextureDelegate>(&source.texture)
        .unwrap()
        .reference
        .clone();

    ServerTextureRef {
        texture,
        transform: source.transform,
        texture_coord_slot: source.texture_coord_slot,
    }
}

pub fn advance_client(ptr: &ReplayClientPtr, bytes: &[u8]) {
    let mut lock = ptr.lock().unwrap();
    handle_next(&mut lock, bytes).unwrap();
}

pub type ReplayClientPtr = Arc<Mutex<ClientState>>;

pub fn make_client_ptr(
    server: Weak<Mutex<ReplayerServerState>>,
) -> ReplayClientPtr {
    let channels = start_blank_stream();
    let maker = Maker {
        server_link: server,
    };
    Arc::new(Mutex::new(ClientState::new(&channels, maker)))
}

trait MyClientCrap {}

// =============================================================================

pub struct ReplayDocDelegate {
    server_link: Weak<Mutex<ReplayerServerState>>,
    pub methods_list: Option<Vec<MethodReference>>,
    pub signals_list: Option<Vec<SignalReference>>,
}

impl ReplayDocDelegate {
    pub fn new(server_link: Weak<Mutex<ReplayerServerState>>) -> Self {
        Self {
            server_link,
            methods_list: None,
            signals_list: None,
        }
    }
}

impl DocumentDelegate for ReplayDocDelegate {
    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientState,
        _args: Vec<Value>,
    ) {
        todo!("Not yet implemented");
    }

    fn on_document_update(
        &mut self,
        client: &mut ClientState,
        update: ClientDocumentUpdate,
    ) {
        let new_methods = update.methods_list.map(|mlist| {
            convert_methods(&client.delegate_lists, Some(mlist)).unwrap()
        });

        let new_signals = update.signals_list.map(|slist| {
            convert_signals(&client.delegate_lists, Some(slist)).unwrap()
        });

        let app_link = self.server_link.upgrade().unwrap();

        let app_lock = app_link.lock().unwrap();

        let server_ptr = app_lock.server_state();

        let mut server_lock = server_ptr.lock().unwrap();

        server_lock.update_document(ServerDocumentUpdate {
            methods_list: new_methods.clone(),
            signals_list: new_signals.clone(),
        });

        self.methods_list = new_methods;
        self.signals_list = new_signals;
    }
}

// =============================================================================

macro_rules! declare_delegate {
    ($id:ident, $reftype:ty, $idtype:ty, $inittype:ty) => {
        pub struct $id {
            #[allow(dead_code)]
            reference: $reftype,
        }

        impl Delegate for $id {
            type IDType = $idtype;
            type InitStateType = $inittype;

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        impl $id {
            fn new(_id: $idtype, reference: $reftype) -> Self {
                Self { reference }
            }
        }
    };
}

declare_delegate!(
    RMethodDelegate,
    MethodReference,
    MethodID,
    ClientMethodState
);

declare_delegate!(
    RSignalDelegate,
    SignalReference,
    SignalID,
    ClientSignalState
);

declare_delegate!(RBufferDelegate, BufferReference, BufferID, BufferState);
declare_delegate!(
    RBufferViewDelegate,
    BufferViewReference,
    BufferViewID,
    ClientBufferViewState
);

declare_delegate!(RSamplerDelegate, SamplerReference, SamplerID, SamplerState);
declare_delegate!(RImageDelegate, ImageReference, ImageID, ClientImageState);
declare_delegate!(
    RTextureDelegate,
    TextureReference,
    TextureID,
    ClientTextureState
);

declare_delegate!(
    RMaterialDelegate,
    MaterialReference,
    MaterialID,
    ClientMaterialState
);

impl UpdatableDelegate for RMaterialDelegate {
    type UpdateStateType = ClientMaterialUpdate;

    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
        convert_material_update(client, state).patch(&self.reference);
    }

    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientDelegateLists,
        _args: Vec<colabrodo_client::mapped_client::ciborium::value::Value>,
    ) {
        todo!("Not yet implemented!");
    }
}

fn convert_material_update(
    client: &mut ClientDelegateLists,
    state: ClientMaterialUpdate,
) -> ServerMaterialStateUpdatable {
    ServerMaterialStateUpdatable {
        pbr_info: state.pbr_info.map(|f| ServerPBRInfo {
            base_color: f.base_color,
            base_color_texture: convert_opt_texture_ref(
                client,
                f.base_color_texture,
            ),
            metallic: f.metallic,
            roughness: f.roughness,
            metal_rough_texture: convert_opt_texture_ref(
                client,
                f.metal_rough_texture,
            ),
        }),
        normal_texture: convert_opt_texture_ref(client, state.normal_texture),
        occlusion_texture: convert_opt_texture_ref(
            client,
            state.occlusion_texture,
        ),
        occlusion_texture_factor: state.occlusion_texture_factor,
        emissive_texture: convert_opt_texture_ref(
            client,
            state.emissive_texture,
        ),
        emissive_factor: state.emissive_factor,
        use_alpha: state.use_alpha,
        alpha_cutoff: state.alpha_cutoff,
        double_sided: state.double_sided,
    }
}

declare_delegate!(
    RGeometryDelegate,
    GeometryReference,
    GeometryID,
    ClientGeometryState
);

declare_delegate!(RLightDelegate, LightReference, LightID, LightState);

impl UpdatableDelegate for RLightDelegate {
    type UpdateStateType = LightStateUpdatable;

    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
        convert_light_update(client, state).patch(&self.reference);
    }

    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientDelegateLists,
        _args: Vec<colabrodo_client::mapped_client::ciborium::value::Value>,
    ) {
        todo!("Not yet implemented!");
    }
}

fn convert_light_update(
    _client: &mut ClientDelegateLists,
    state: LightStateUpdatable,
) -> ServerLightStateUpdatable {
    ServerLightStateUpdatable {
        color: state.color,
        intensity: state.intensity,
    }
}

declare_delegate!(RTableDelegate, TableReference, TableID, ClientTableState);

impl UpdatableDelegate for RTableDelegate {
    type UpdateStateType = ClientTableUpdate;

    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
        convert_table_update(client, state).patch(&self.reference);
    }

    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientDelegateLists,
        _args: Vec<colabrodo_client::mapped_client::ciborium::value::Value>,
    ) {
        todo!("Not yet implemented!");
    }
}

fn convert_table_update(
    client: &mut ClientDelegateLists,
    state: ClientTableUpdate,
) -> ServerTableStateUpdatable {
    ServerTableStateUpdatable {
        meta: state.meta,
        methods_list: convert_methods(client, state.methods_list),
        signals_list: convert_signals(client, state.signals_list),
    }
}

declare_delegate!(RPlotDelegate, PlotReference, PlotID, ClientPlotState);

impl UpdatableDelegate for RPlotDelegate {
    type UpdateStateType = ClientPlotUpdate;

    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
        convert_plot_update(client, state).patch(&self.reference);
    }

    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientDelegateLists,
        _args: Vec<colabrodo_client::mapped_client::ciborium::value::Value>,
    ) {
        todo!("Not yet implemented!");
    }
}

fn convert_plot_update(
    client: &mut ClientDelegateLists,
    state: ClientPlotUpdate,
) -> ServerPlotStateUpdatable {
    ServerPlotStateUpdatable {
        table: state
            .table
            .map(|tid| {
                client.table_list.find_as::<RTableDelegate>(&tid).unwrap()
            })
            .map(|f| f.reference.clone()),
        methods_list: convert_methods(client, state.methods_list),
        signals_list: convert_signals(client, state.signals_list),
    }
}

declare_delegate!(
    REntityDelegate,
    EntityReference,
    EntityID,
    ClientEntityState
);

impl UpdatableDelegate for REntityDelegate {
    type UpdateStateType = ClientEntityUpdate;

    fn on_update(
        &mut self,
        client: &mut ClientDelegateLists,
        state: Self::UpdateStateType,
    ) {
        convert_entity_update(client, state).patch(&self.reference);
    }

    fn on_signal(
        &mut self,
        _id: SignalID,
        _client: &mut ClientDelegateLists,
        _args: Vec<colabrodo_client::mapped_client::ciborium::value::Value>,
    ) {
        todo!("Not yet implemented!");
    }
}

fn convert_entity_update(
    client: &mut ClientDelegateLists,
    state: ClientEntityUpdate,
) -> ServerEntityStateUpdatable {
    ServerEntityStateUpdatable {
        parent: state.parent.map(|pid| {
            client
                .entity_list
                .find_as::<REntityDelegate>(&pid)
                .unwrap()
                .reference
                .clone()
        }),
        transform: state.transform,
        representation: convert_representation(client, state.representation),
        lights: convert_lights(client, state.lights),
        tables: convert_tables(client, state.tables),
        plots: convert_plots(client, state.plots),
        tags: state.tags,
        methods_list: convert_methods(client, state.methods_list),
        signals_list: convert_signals(client, state.signals_list),
        influence: state.influence,
        visible: state.visible,
    }
}
