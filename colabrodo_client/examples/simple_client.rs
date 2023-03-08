use clap::Parser;
use colabrodo_client::client::*;
use colabrodo_client::components::*;
//use colabrodo_client::table::BasicTable;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;

/*
// =============================================================================

/// A cleaner way to print out type names
fn get_type_name<T>() -> String {
    let mut tname = std::any::type_name::<T>();

    //find '<' and chop string at that point

    if let Some(index) = tname.find('<') {
        tname = &tname[..index];
    }

    if let Some(index) = tname.rfind(':') {
        tname = &tname[(index + 1)..];
    }

    tname.to_string()
}

/// A list of components.
///
/// We use the provided basic list and simply add some functions to intercept and print changes for exposition.
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
    methods: ExampleComponentList<ClientMethodState>,
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

    subscribed_tables: HashMap<NooID, ManagedTable<BasicTable>>,
    to_server: Option<tokio::sync::mpsc::Sender<OutgoingMessage>>,
}

struct ExampleStateArgument {}

#[derive(Debug)]
struct ExampleStateCommand {}

impl UserClientState for ExampleState {
    type MethodL = ExampleComponentList<ClientMethodState>;
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
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        Self {
            to_server: Some(to_server),
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
        println!("Signal invoked {signal:?}");

        // check if any signal should be channeled to a local table
        for v in self.subscribed_tables.values_mut() {
            if v.check_relevant_signal(&signal) {
                v.consume_relevant_signal(signal);
                dump_table(v);
                break;
            }
        }
    }
    fn on_method_reply(&mut self, method_reply: MessageMethodReply) {
        println!("Method reply: {method_reply:?}");

        // check if any of the replies belong to a subscribed table
        for v in self.subscribed_tables.values_mut() {
            if v.check_relevant_method(&method_reply) {
                v.consume_relevant_method(method_reply);

                dump_table(v);

                break;
            }
        }
    }
    fn on_document_ready(&mut self) {
        println!("Document is ready, subscribing to any tables...");

        // Go through all available tables and subscribe

        let info = ResolvedTableIDs::new(self);

        for (k, v) in self.tables.list.component_map() {
            let mut managed_table = ManagedTable::new(
                *k,
                info.clone(),
                BasicTable::default(),
                self.to_server.as_ref().unwrap().clone(),
            );

            if managed_table.subscribe().is_none() {
                println!("Unable to subscribe to table");
            } else {
                println!(
                    "Subscribed to table: {}",
                    v.name.as_ref().unwrap_or(&k.to_string())
                );
            }

            self.subscribed_tables.insert(*k, managed_table);
        }
    }
}

// =============================================================================

/// Dump a CBOR value in a pretty way
fn print_value(value: &ciborium::value::Value) {
    match value {
        ciborium::value::Value::Integer(i) => print!("{i:?}"),
        ciborium::value::Value::Bytes(b) => print!("{b:02X?}"),
        ciborium::value::Value::Float(f) => print!("{f}"),
        ciborium::value::Value::Text(t) => print!("{t}"),
        ciborium::value::Value::Bool(b) => print!("{b}"),
        ciborium::value::Value::Null => print!("Null"),
        ciborium::value::Value::Tag(t, b) => {
            print!("<Tag {t}>");
            print_value(b);
        }
        ciborium::value::Value::Array(a) => {
            print!("Array [");
            for i in a {
                print_value(i);
                print!(",");
            }
            print!("]");
        }
        ciborium::value::Value::Map(m) => {
            print!("Map {{");
            for (k, v) in m {
                print_value(k);
                print!(":");
                print_value(v);
                print!(",");
            }
            print!("}}")
        }
        _ => todo!(),
    }
}

/// Print a whole table to stdout
fn dump_table(t: &ManagedTable<BasicTable>) {
    println!("Data:");
    for h in t.table_data().header() {
        print!("{}\t", h.name)
    }

    println!();

    for (_key, value) in t.table_data().current_data() {
        for col in value {
            print_value(col);
            print!("\t");
        }
        println!();
    }
}

 */

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

fn callback(_client: &mut ClientState, msg: &FromServer) {
    println!("Message: {msg:?}");
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Arguments::parse();

    let channels = ClientChannels::new();

    let state = ClientState::new_with_callback(&channels, callback);

    start_client(
        args.address,
        "Simple Client".to_string(),
        state.clone(),
        channels,
    )
    .await
    .unwrap();
}
