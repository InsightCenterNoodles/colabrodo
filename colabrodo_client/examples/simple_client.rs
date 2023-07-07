use std::any::Any;
use std::collections::BTreeMap;
use std::collections::HashMap;

use ciborium::value::Value;
use clap::Parser;
use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_client::components::ClientTableState;
use colabrodo_client::components::ClientTableUpdate;
use colabrodo_client::delegate::*;
use colabrodo_client::table::*;
use colabrodo_common::nooid::TableID;

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

/// Print out a CBOR value in a pretty way
fn dump_value(v: &Value) {
    match v {
        Value::Integer(x) => print!("{}", i128::from(*x)),
        Value::Bytes(b) => print!("{b:?}"),
        Value::Float(f) => print!("{f}"),
        Value::Text(s) => print!("{s}"),
        Value::Bool(b) => print!("{b}"),
        Value::Null => print!("Null"),
        Value::Tag(x, y) => {
            print!("Tag: {x};");
            dump_value(y)
        }
        Value::Array(a) => {
            print!("[");
            for v in a {
                dump_value(v);
            }
            print!("]");
        }
        Value::Map(m) => {
            print!("{{");
            for (k, v) in m {
                print!("(");
                dump_value(k);
                print!(", ");
                dump_value(v);
                print!(")");
            }
            print!("}}");
        }
        _ => todo!(),
    }
}

/// A table that prints out changes
struct ReportingTable {
    header: Vec<TableColumnInfo>,
    data: BTreeMap<i64, Vec<Value>>,
    selections: HashMap<String, Selection>,
}

impl ReportingTable {
    /// Print the whole table
    fn dump_table(&self) {
        print!("ID\t");
        for c in &self.header {
            print!("{}\t", c.name);
        }
        println!();
        for (k, v) in &self.data {
            print!("{k}\t");

            for val in v {
                dump_value(val);
                print!("\t");
            }

            println!();
        }
    }
}

/// Implement required trait for our table. This will just print out changes.
impl TableDataStorage for ReportingTable {
    fn on_init_data(&mut self, init_data: TableInitData) {
        println!("Initialize Table: {init_data:?}");
        self.header = init_data.columns;

        for (k, v) in init_data.keys.into_iter().zip(init_data.data.into_iter())
        {
            self.data.insert(k, v);
        }

        self.dump_table();
    }

    fn update_data(
        &mut self,
        keys: Vec<i64>,
        updated_data: Vec<Vec<ciborium::value::Value>>,
    ) {
        for (k, v) in keys.into_iter().zip(updated_data.into_iter()) {
            self.data.insert(k, v);
        }

        self.dump_table();
    }

    fn delete_data(&mut self, keys: Vec<i64>) {
        for k in keys {
            self.data.remove(&k);
        }

        self.dump_table();
    }

    fn update_selection(
        &mut self,
        selection: colabrodo_common::table::Selection,
    ) {
        let sel_id = selection.name.clone();

        if selection.row_ranges.is_none() && selection.rows.is_none() {
            self.selections.remove(&sel_id);
            return;
        }

        self.selections.insert(sel_id, selection);
    }

    fn clear(&mut self) {
        self.selections.clear();
        self.data.clear();

        self.dump_table();
    }
}

// =============================================================================

#[derive(Debug, Default)]
struct MyMaker {}

impl DelegateMaker for MyMaker {
    fn make_table(
        &mut self,
        id: TableID,
        _state: ClientTableState,
        _client: &mut ClientDelegateLists,
    ) -> Box<TableDelegate> {
        Box::new(MyTableDelegate::new(id))
    }
}

struct MyTableDelegate {
    pre_made_delegate: AdvTableDelegate,
}

impl MyTableDelegate {
    fn new(id: TableID) -> Self {
        Self {
            pre_made_delegate: AdvTableDelegate::new(id),
        }
    }
}

impl Delegate for MyTableDelegate {
    type IDType = TableID;
    type InitStateType = ClientTableState;

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl UpdatableDelegate for MyTableDelegate {
    type UpdateStateType = ClientTableUpdate;

    fn on_update(&mut self, state: Self::UpdateStateType) {
        self.pre_made_delegate.on_update(state);
    }

    fn on_signal(
        &mut self,
        id: colabrodo_common::nooid::SignalID,
        client: &mut ClientDelegateLists,
        args: Vec<ciborium::value::Value>,
    ) {
        self.pre_made_delegate.on_signal(id, client, args);
    }

    fn on_method_reply(
        &mut self,
        client: &mut ClientDelegateLists,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
        self.pre_made_delegate
            .on_method_reply(client, invoke_id, reply);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Arguments::parse();

    // Create required channels for the client
    let channels =
        start_client_stream(args.address.into(), "Simple Client".into())
            .await
            .unwrap();

    let maker = MyMaker::default();

    // Launch a client thread to handle those channels
    launch_client_worker_thread(channels, maker);
}
