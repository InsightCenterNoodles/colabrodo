use std::collections::HashMap;

use colabrodo_common::tf_to_cbor;
use colabrodo_common::value_tools::*;
use colabrodo_server::server_state::MethodException;
use colabrodo_server::{server::*, server_messages::*, table::*};

/// Create a rock-simple table for export
fn make_init_table() -> BasicTable {
    let header = vec![
        TableColumnInfo {
            name: "A".to_string(),
            type_: "REAL".to_string(),
        },
        TableColumnInfo {
            name: "B".to_string(),
            type_: "REAL".to_string(),
        },
        TableColumnInfo {
            name: "C".to_string(),
            type_: "TEXT".to_string(),
        },
    ];

    let row1 = tf_to_cbor!(1.0_f32, 2.0_f32, "Row 1".to_string());
    let row2 = tf_to_cbor!(3.0_f32, 4.0_f32, "Row 2".to_string());
    let row3 = tf_to_cbor!(5.0_f32, 6.0_f32, "Row 3".to_string());

    let data = vec![row1, row2, row3];

    BasicTable::new(header, data)
}

// =============================================================================

struct PublishTableExample {
    state: ServerState,
    table_store: TableStore<BasicTable>,
    client_map: HashMap<uuid::Uuid, ClientRecord>,
}

impl UserServerState for PublishTableExample {
    fn mut_state(&mut self) -> &mut ServerState {
        &mut self.state
    }

    fn state(&self) -> &ServerState {
        &self.state
    }

    fn invoke(
        &mut self,
        method: ComponentReference<MethodState>,
        context: InvokeObj,
        client_id: uuid::Uuid,
        args: Vec<ciborium::value::Value>,
    ) -> MethodResult {
        // Check if any table methods have been called on an exported table.
        // If so, channel it to the table for handling

        let c = self.client_map.get(&client_id).unwrap();

        if self.table_store.is_relevant_method(&method, &context) {
            return self
                .table_store
                .consume_relevant_method(c, &method, context, args);
        }

        Err(MethodException::method_not_found(None))
    }
}

impl AsyncServer for PublishTableExample {
    type CommandType = DefaultCommand;

    fn initialize_state(&mut self, state: &mut ServerState) {
        let mut state = ServerState::new();

        // Create a new table ID
        let table = state.tables.new_component(ServerTableState {
            name: Some("Example Table".to_string()),
            mutable: ServerTableStateUpdatable::default(),
        });

        // Build table methods and signals
        // Note that this will create and resolve the methods, so don't call this more than once; instead clone or move the created object around.
        let table_methods_signals = CreateTableMethods::new(&mut state);

        // Create a managed table for export, and hook up to our new table ID
        let table_store = TableStore::new(
            &mut state,
            table_methods_signals,
            table.clone(),
            make_init_table(),
        );

        Self {
            state,
            table_store,
            client_map: Default::default(),
        }
    }

    fn client_connected(&mut self, record: ClientRecord) {
        // We need to know which client is connecting so we can let them subscribe to a table
        self.client_map.insert(record.id, record);
    }

    fn client_disconnected(&mut self, client_id: uuid::Uuid) {
        // We need to make sure that the table system doesn't try to keep sending info to clients that have gone away...
        self.table_store.forget_client(client_id);
        self.client_map.remove(&client_id);
    }
}

// =============================================================================

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Connect clients to localhost:50000");
    let opts = ServerOptions::default();
    server_main::<PublishTableExample>(opts, NoInit {}).await;
}
