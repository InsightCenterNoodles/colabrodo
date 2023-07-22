use colabrodo_common::tf_to_cbor;
use colabrodo_common::value_tools::*;
use colabrodo_server::{server::*, server_messages::*, table_system::*};

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

/// Struct to hold our example data
struct ExampleState {
    _state: ServerStatePtr,
    _table_system: TableSystemPtr,
}

/// Set up the example
fn setup(state: ServerStatePtr) -> ExampleState {
    log::debug!("Initializing table state");

    // Create a new table component
    let table = {
        let mut lock = state.lock().unwrap();

        lock.tables.new_component(ServerTableState {
            name: Some("Example Table".to_string()),
            mutable: ServerTableStateUpdatable::default(),
        })
    };

    // Initialize the table handling system
    let table_system = {
        let mut lock = state.lock().unwrap();

        TableSystem::new(&mut lock)
    };

    // Link the data with the table component
    {
        let mut lock = state.lock().unwrap();
        let mut table_lock = table_system.lock().unwrap();

        table_lock.register_table(table, &mut lock, make_init_table());
    }

    ExampleState {
        _state: state,
        _table_system: table_system,
    }
}

// =============================================================================

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Connect clients to localhost:50000");
    let opts = ServerOptions::default();

    let state = ServerState::new();

    let _ = setup(state.clone());

    server_main(opts, state).await;
}
