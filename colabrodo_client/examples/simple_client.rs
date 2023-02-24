use std::fmt::Debug;

use colabrodo_client::client::*;
use colabrodo_client::components::*;

use clap::Parser;

// =============================================================================

fn get_type_name<T>() -> String {
    let mut tname = std::any::type_name::<T>();

    //find '<' and chop string at that point

    if let Some(index) = tname.find('<') {
        tname = &tname[..index];
    }

    if let Some(index) = tname.rfind(":") {
        tname = &tname[(index + 1)..];
    }

    tname.to_string()
}

#[derive(Debug)]
struct ExampleComponentList<State: NamedComponent> {
    ty: String,
    list: BasicComponentList<State>,
}

impl<State: NamedComponent> Default for ExampleComponentList<State> {
    fn default() -> Self {
        Self {
            ty: get_type_name::<State>(),
            list: Default::default(),
        }
    }
}

impl<State: NamedComponent> ComponentList<State> for ExampleComponentList<State>
where
    State: Debug,
{
    fn on_create(&mut self, id: NooID, state: State) {
        println!("Create {}: {id}: {state:?}", self.ty);
        self.list.on_create(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        println!("Delete {}: {id}", self.ty);
        self.list.on_delete(id);
    }

    fn find(&self, id: &NooID) -> Option<&State> {
        self.list.find(id)
    }

    fn get_id_by_name(&self, name: &str) -> Option<&NooID> {
        self.list.get_id_by_name(name)
    }
}

impl<State: NamedComponent> UpdatableComponentList<State>
    for ExampleComponentList<State>
where
    State: UpdatableWith + Debug,
    State::Substate: Debug,
{
    fn on_update(&mut self, id: NooID, update: State::Substate) {
        println!("Update {}: {id}: {update:?}", self.ty);
        self.list.on_update(id, update);
    }
}

// =============================================================================

#[derive(Debug, Default)]
struct ExampleState {
    methods: ExampleComponentList<MethodState>,
    signals: ExampleComponentList<SignalState>,
    buffers: ExampleComponentList<BufferState>,
    buffer_views: ExampleComponentList<ClientBufferViewState>,
    samplers: ExampleComponentList<SamplerState>,
    images: ExampleComponentList<ClientImageState>,
    textures: ExampleComponentList<ClientTextureState>,
    materials: ExampleComponentList<ClientMaterialState>,
    geometries: ExampleComponentList<ClientGeometryState>,
    lights: ExampleComponentList<LightState>,
    tables: ExampleComponentList<ClientTableState>,
    plots: ExampleComponentList<ClientPlotState>,
    entities: ExampleComponentList<ClientEntityState>,

    doc: ClientDocumentUpdate,
}

struct ExampleStateArgument {}

#[derive(Debug)]
struct ExampleStateCommand {}

impl UserClientState for ExampleState {
    type MethodL = ExampleComponentList<MethodState>;
    type SignalL = ExampleComponentList<SignalState>;
    type BufferL = ExampleComponentList<BufferState>;
    type BufferViewL = ExampleComponentList<ClientBufferViewState>;
    type SamplerL = ExampleComponentList<SamplerState>;
    type ImageL = ExampleComponentList<ClientImageState>;
    type TextureL = ExampleComponentList<ClientTextureState>;
    type MaterialL = ExampleComponentList<ClientMaterialState>;
    type GeometryL = ExampleComponentList<ClientGeometryState>;
    type LightL = ExampleComponentList<LightState>;
    type TableL = ExampleComponentList<ClientTableState>;
    type PlotL = ExampleComponentList<ClientPlotState>;
    type EntityL = ExampleComponentList<ClientEntityState>;

    type CommandType = ExampleStateCommand;
    type ArgumentType = ExampleStateArgument;

    fn new(
        _a: Self::ArgumentType,
        _to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        Self {
            ..Default::default()
        }
    }

    fn method_list(&mut self) -> &mut Self::MethodL {
        &mut self.methods
    }

    fn signal_list(&mut self) -> &mut Self::SignalL {
        &mut self.signals
    }

    fn buffer_list(&mut self) -> &mut Self::BufferL {
        &mut self.buffers
    }

    fn buffer_view_list(&mut self) -> &mut Self::BufferViewL {
        &mut self.buffer_views
    }

    fn sampler_list(&mut self) -> &mut Self::SamplerL {
        &mut self.samplers
    }

    fn image_list(&mut self) -> &mut Self::ImageL {
        &mut self.images
    }

    fn texture_list(&mut self) -> &mut Self::TextureL {
        &mut self.textures
    }

    fn material_list(&mut self) -> &mut Self::MaterialL {
        &mut self.materials
    }

    fn geometry_list(&mut self) -> &mut Self::GeometryL {
        &mut self.geometries
    }

    fn light_list(&mut self) -> &mut Self::LightL {
        &mut self.lights
    }

    fn table_list(&mut self) -> &mut Self::TableL {
        &mut self.tables
    }

    fn plot_list(&mut self) -> &mut Self::PlotL {
        &mut self.plots
    }

    fn entity_list(&mut self) -> &mut Self::EntityL {
        &mut self.entities
    }

    fn document_update(&mut self, update: ClientDocumentUpdate) {
        self.doc.update(update);
    }

    fn on_signal_invoke(&mut self, signal: ClientMessageSignalInvoke) {
        println!("Signal invoked {signal:?}")
    }
    fn on_method_reply(&mut self, method_reply: MessageMethodReply) {
        println!("Method reply: {method_reply:?}")
    }
    fn on_document_ready(&mut self) {
        println!("Document is ready!");
    }
}

// =============================================================================

#[derive(Parser)]
#[command(name = "simple_client")]
#[command(version = "0.1")]
#[command(about = "Example NOODLES client", long_about = None)]
pub struct Arguments {
    /// Host address to connect to
    #[arg(default_value_t = String::from("ws://localhost:50000"))]
    pub address: String,
}

// =============================================================================

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Arguments::parse();

    start_client::<ExampleState>(
        args.address,
        "Simple Client".to_string(),
        ExampleStateArgument {},
    )
    .await
    .unwrap()
}
