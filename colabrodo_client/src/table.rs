use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex, Weak},
};

use colabrodo_common::{
    arg_to_tuple, common::strings, nooid::*, value_tools::*,
};

pub use colabrodo_common::table::*;

use crate::client::{invoke_method, ClientState, InvokeContext, SignalRecv};

/// Interface for a client-side table
pub trait TableDataStorage {
    /// Called when the table should be initialized
    fn on_init_data(&mut self, init_data: TableInitData);

    /// Called when the table should be updated
    fn update_data(&mut self, keys: Vec<i64>, updated_data: Vec<Vec<Value>>);

    /// Called when keys should be removed from the table
    fn delete_data(&mut self, keys: Vec<i64>);

    /// Called when a selection should be updated
    fn update_selection(&mut self, selection: Selection);

    /// Called when the table should be cleared
    fn clear(&mut self);
}

// =============================================================================

/// A structure to resolve IDs of important table signals and methods
#[derive(Debug, Clone)]
pub struct ResolvedTableIDs {
    sig_reset: Option<SignalID>,
    sig_updated: Option<SignalID>,
    sig_row_remove: Option<SignalID>,
    sig_selection_update: Option<SignalID>,

    mthd_subscribe: Option<MethodID>,
    mthd_insert: Option<MethodID>,
    mthd_update: Option<MethodID>,

    mthd_remove: Option<MethodID>,
    mthd_clear: Option<MethodID>,
    mthd_update_selection: Option<MethodID>,
}

impl ResolvedTableIDs {
    /// Resolve ids
    pub fn new(state: &ClientState) -> Self {
        Self {
            sig_reset: state.signal_list.get_id_by_name(strings::SIG_TBL_RESET),
            sig_updated: state
                .signal_list
                .get_id_by_name(strings::SIG_TBL_UPDATED),
            sig_row_remove: state
                .signal_list
                .get_id_by_name(strings::SIG_TBL_ROWS_REMOVED),
            sig_selection_update: state
                .signal_list
                .get_id_by_name(strings::SIG_TBL_SELECTION_UPDATED),
            mthd_subscribe: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_SUBSCRIBE),
            mthd_insert: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_INSERT),
            mthd_update: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_UPDATE),
            mthd_remove: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_REMOVE),
            mthd_clear: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_CLEAR),
            mthd_update_selection: state
                .method_list
                .get_id_by_name(strings::MTHD_TBL_UPDATE_SELECTION),
        }
    }
}

// =============================================================================

///
/// Warning, this function takes a lock quickly on state to get things setup, so make sure to avoid deadlocks.
pub async fn connect_table<T: TableDataStorage + Send + 'static>(
    state: Arc<Mutex<ClientState>>,
    remote_table_id: TableID,
    table_data: T,
) -> Option<ManagedTablePtr<T>> {
    let resolved_ids = {
        let lock = state.lock().unwrap();
        ResolvedTableIDs::new(&lock)
    };

    if resolved_ids.mthd_subscribe.is_none() {
        return None;
    }

    log::debug!("Subscribing to table...");

    let init_data = invoke_method(
        state.clone(),
        resolved_ids.mthd_subscribe.unwrap(),
        InvokeContext::Table(remote_table_id),
        vec![],
    )
    .await
    .ok()?;

    log::debug!("Subscribed, creating table...");

    let init_data =
        from_cbor(init_data?).unwrap_or_else(|_| TableInitData::default());

    let managed_table = ManagedTable::new(
        state.clone(),
        remote_table_id,
        table_data,
        init_data,
    );

    signal_watcher(state, managed_table.clone()).await;

    Some(managed_table)
}

/// Ask to insert data into the remote table
pub async fn ask_insert<T: TableDataStorage>(
    table: ManagedTablePtr<T>,
    rows: Vec<Vec<Value>>,
) -> Option<()> {
    let (method_id, remote_table_id, state) = {
        let lock = table.lock().unwrap();
        let method = lock.resolved_ids.mthd_insert?;
        let state = lock.state.upgrade()?;
        (method, lock.remote_table_id, state)
    };

    let res = invoke_method(
        state,
        method_id,
        InvokeContext::Table(remote_table_id),
        vec![to_cbor(&rows)],
    )
    .await
    .ok();

    match res {
        Some(_) => Some(()),
        None => None,
    }
}

/// Ask to update keys on the remote table
pub async fn ask_update<T: TableDataStorage>(
    table: ManagedTablePtr<T>,
    keys: Vec<i64>,
    rows: Vec<Vec<Value>>,
) -> Option<()> {
    let (method_id, remote_table_id, state) = {
        let lock = table.lock().unwrap();
        let method = lock.resolved_ids.mthd_update?;
        let state = lock.state.upgrade()?;
        (method, lock.remote_table_id, state)
    };

    let res = invoke_method(
        state,
        method_id,
        InvokeContext::Table(remote_table_id),
        vec![to_cbor(&keys), to_cbor(&rows)],
    )
    .await
    .ok();

    match res {
        Some(_) => Some(()),
        None => None,
    }
}

/// Ask to remove keys from the remote table
pub async fn ask_remove<T: TableDataStorage>(
    table: ManagedTablePtr<T>,
    keys: Vec<i64>,
) -> Option<()> {
    let (method_id, remote_table_id, state) = {
        let lock = table.lock().unwrap();
        let method = lock.resolved_ids.mthd_remove?;
        let state = lock.state.upgrade()?;
        (method, lock.remote_table_id, state)
    };

    let res = invoke_method(
        state,
        method_id,
        InvokeContext::Table(remote_table_id),
        vec![to_cbor(&keys)],
    )
    .await
    .ok();

    match res {
        Some(_) => Some(()),
        None => None,
    }
}

/// Ask to clear the table
pub async fn ask_clear<T: TableDataStorage>(
    table: ManagedTablePtr<T>,
) -> Option<()> {
    let (method_id, remote_table_id, state) = {
        let lock = table.lock().unwrap();
        let method = lock.resolved_ids.mthd_clear?;
        let state = lock.state.upgrade()?;
        (method, lock.remote_table_id, state)
    };

    let res = invoke_method(
        state,
        method_id,
        InvokeContext::Table(remote_table_id),
        vec![],
    )
    .await
    .ok();

    match res {
        Some(_) => Some(()),
        None => None,
    }
}

/// Ask to update a specific selection
pub async fn ask_update_selection<T: TableDataStorage>(
    table: ManagedTablePtr<T>,
    selection: Selection,
) -> Option<()> {
    let (method_id, remote_table_id, state) = {
        let lock = table.lock().unwrap();
        let method = lock.resolved_ids.mthd_update_selection?;
        let state = lock.state.upgrade()?;
        (method, lock.remote_table_id, state)
    };

    let res = invoke_method(
        state,
        method_id,
        InvokeContext::Table(remote_table_id),
        vec![to_cbor(&selection)],
    )
    .await
    .ok();

    match res {
        Some(_) => Some(()),
        None => None,
    }
}

// =============================================================================

async fn exec_listener<F, T>(
    t: Option<SignalRecv>,
    table: ManagedTablePtr<T>,
    f: F,
) where
    F: Fn(&mut ManagedTable<T>, Vec<Value>) -> Option<()>,
    T: TableDataStorage + Send,
{
    if let Some(mut channel) = t {
        while let Ok(msg) = channel.recv().await {
            let mut lock = table.lock().unwrap();
            f(&mut lock, msg);
        }
    }
}

async fn signal_watcher<T: TableDataStorage + Send + 'static>(
    state: Arc<Mutex<ClientState>>,
    table: ManagedTablePtr<T>,
) -> Option<()> {
    let resolved_ids = table.lock().unwrap().resolved_ids.clone();
    let table_id = table.lock().unwrap().remote_table_id;

    let (reset_stream, remove_stream, update_stream, sel_update_stream) = {
        let mut lock = state.lock().unwrap();
        let reset_stream = lock
            .table_list
            .subscribe_signal(table_id, resolved_ids.sig_reset?);

        let remove_stream = lock
            .table_list
            .subscribe_signal(table_id, resolved_ids.sig_row_remove?);

        let update_stream = lock
            .table_list
            .subscribe_signal(table_id, resolved_ids.sig_updated?);

        let sel_update_stream = lock
            .table_list
            .subscribe_signal(table_id, resolved_ids.sig_selection_update?);

        (
            reset_stream,
            remove_stream,
            update_stream,
            sel_update_stream,
        )
    };

    tokio::spawn(exec_listener(reset_stream, table.clone(), |l, _| {
        l.table_storage.clear();
        Some(())
    }));

    tokio::spawn(exec_listener(remove_stream, table.clone(), |l, msg| {
        let keys = arg_to_tuple!(msg, Vec<i64>)?;
        l.table_storage.delete_data(keys.0);
        Some(())
    }));

    tokio::spawn(exec_listener(update_stream, table.clone(), |l, msg| {
        let (keys, rows) = arg_to_tuple!(msg, Vec<i64>, Vec<Vec<Value>>)?;
        l.table_storage.update_data(keys, rows);
        Some(())
    }));

    tokio::spawn(exec_listener(sel_update_stream, table.clone(), |l, msg| {
        let selection = arg_to_tuple!(msg, Selection)?;
        l.table_storage.update_selection(selection.0);
        Some(())
    }));

    Some(())
}

// =============================================================================

/// A table managed by the library
///
/// This structure will automatically ask to update remote data and react to changes.
pub struct ManagedTable<T: TableDataStorage> {
    state: Weak<Mutex<ClientState>>,
    remote_table_id: TableID,
    table_storage: T,
    resolved_ids: ResolvedTableIDs,
}

type ManagedTablePtr<T> = Arc<Mutex<ManagedTable<T>>>;

impl<T: TableDataStorage> ManagedTable<T> {
    /// Create a new managed table with a given generic table class
    ///
    /// Consumes the remote table ID to watch, a list of resolved signals and methods that relate to table operations, an instance of data storage to manage, and a channel to the server
    fn new(
        state: Arc<Mutex<ClientState>>,
        remote_table_id: TableID,
        mut table_storage: T,
        init_data: TableInitData,
    ) -> Arc<Mutex<Self>> {
        let lock = state.lock().unwrap();

        table_storage.on_init_data(init_data);

        Arc::new(Mutex::new(Self {
            state: Arc::downgrade(&state),
            remote_table_id,
            table_storage,
            resolved_ids: ResolvedTableIDs::new(&lock),
        }))
    }
}

// =============================================================================

/// A simple data storage class for use with managed tables.
///
/// This class can be used directly, or used as a model for custom user classes
#[derive(Debug, Default)]
pub struct BasicTable {
    header: Vec<TableColumnInfo>,
    data: BTreeMap<i64, Vec<Value>>,
    selections: HashMap<String, Selection>,
}

impl BasicTable {
    /// Get the current header
    pub fn header(&self) -> &Vec<TableColumnInfo> {
        &self.header
    }

    /// Get a reference to current data
    pub fn current_data(&self) -> &BTreeMap<i64, Vec<Value>> {
        &self.data
    }
}

impl TableDataStorage for BasicTable {
    fn on_init_data(&mut self, init_data: TableInitData) {
        self.header = init_data.columns;

        for (k, v) in init_data.keys.into_iter().zip(init_data.data.into_iter())
        {
            self.data.insert(k, v);
        }

        if let Some(s) = init_data.selections {
            for selection in s {
                self.selections.insert(selection.name.clone(), selection);
            }
        }
    }

    fn update_data(&mut self, keys: Vec<i64>, updated_data: Vec<Vec<Value>>) {
        for (k, v) in keys.into_iter().zip(updated_data.into_iter()) {
            self.data.insert(k, v);
        }
    }

    fn delete_data(&mut self, keys: Vec<i64>) {
        for k in keys {
            self.data.remove(&k);
        }
    }

    fn update_selection(&mut self, selection: Selection) {
        let is_delete = selection
            .row_ranges
            .as_ref()
            .map(|v| v.is_empty())
            .or(selection.rows.as_ref().map(|v| v.is_empty()));

        if is_delete.unwrap_or(true) {
            self.selections.remove(&selection.name);
        } else {
            self.selections.insert(selection.name.clone(), selection);
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.selections.clear();
    }
}
