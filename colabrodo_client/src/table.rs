use std::collections::{BTreeMap, HashMap};

use colabrodo_common::{
    arg_to_tuple,
    client_communication::{ClientInvokeMessage, InvokeIDType},
    common::strings,
    nooid::NooID,
    server_communication::MessageMethodReply,
    table::*,
    value_tools::*,
};

use crate::{
    client::{ComponentList, OutgoingMessage, UserClientState},
    components::ClientMessageSignalInvoke,
};

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
    sig_reset: Option<NooID>,
    sig_updated: Option<NooID>,
    sig_row_remove: Option<NooID>,
    sig_selection_update: Option<NooID>,

    mthd_subscribe: Option<NooID>,
    mthd_insert: Option<NooID>,
    mthd_update: Option<NooID>,

    mthd_remove: Option<NooID>,
    mthd_clear: Option<NooID>,
    mthd_update_selection: Option<NooID>,
}

impl ResolvedTableIDs {
    /// Resolve ids
    pub fn new(state: &mut impl UserClientState) -> Self {
        Self {
            sig_reset: state
                .signal_list()
                .get_id_by_name(strings::SIG_TBL_RESET)
                .copied(),
            sig_updated: state
                .signal_list()
                .get_id_by_name(strings::SIG_TBL_UPDATED)
                .copied(),
            sig_row_remove: state
                .signal_list()
                .get_id_by_name(strings::SIG_TBL_ROWS_REMOVED)
                .copied(),
            sig_selection_update: state
                .signal_list()
                .get_id_by_name(strings::SIG_TBL_SELECTION_UPDATED)
                .copied(),
            mthd_subscribe: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_SUBSCRIBE)
                .copied(),
            mthd_insert: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_INSERT)
                .copied(),
            mthd_update: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_UPDATE)
                .copied(),
            mthd_remove: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_REMOVE)
                .copied(),
            mthd_clear: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_CLEAR)
                .copied(),
            mthd_update_selection: state
                .method_list()
                .get_id_by_name(strings::MTHD_TBL_UPDATE_SELECTION)
                .copied(),
        }
    }
}

// =============================================================================

type MethodHandler<T> = fn(&mut ManagedTable<T>, MessageMethodReply);

/// A table managed by the library
///
/// This structure will automatically ask to update remote data and react to changes.
/// In order to accomplish this, however, this class must have access to incoming signals and method replies. For incoming messages, be sure to check if these are relevant to the table with [check_relevant_method] and [check_relevant_signal]; if relevant, call the corresponding consume function.
pub struct ManagedTable<T: TableDataStorage> {
    remote_table_id: NooID,
    table_data: T,
    to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    resolved_ids: ResolvedTableIDs,
    methods_in_flight: HashMap<uuid::Uuid, MethodHandler<T>>,
}

impl<T: TableDataStorage + std::fmt::Debug> std::fmt::Debug
    for ManagedTable<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedTable")
            .field("remote_table_id", &self.remote_table_id)
            .field("table_data", &self.table_data)
            .field("to_server", &self.to_server)
            .field("resolved_ids", &self.resolved_ids)
            .finish()
    }
}

impl<T: TableDataStorage> ManagedTable<T> {
    /// Create a new managed table with a given generic table class
    ///
    /// Consumes the remote table ID to watch, a list of resolved signals and methods that relate to table operations, an instance of data storage to manage, and a channel to the server
    pub fn new(
        remote_table_id: NooID,
        resolved_ids: ResolvedTableIDs,
        table_data: T,
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        Self {
            remote_table_id,
            table_data,
            to_server,
            resolved_ids,
            methods_in_flight: HashMap::new(),
        }
    }

    /// Get the current data storage
    pub fn table_data(&self) -> &T {
        &self.table_data
    }

    fn on_subscribe_reply(&mut self, method: MessageMethodReply) {
        if let Some(v) = method.result {
            let init: TableInitData =
                from_cbor(v).unwrap_or_else(|_| TableInitData::default());

            log::debug!("Subscribed to table, got init data");

            self.table_data.on_init_data(init);
        }
    }

    /// Ask to subscribe to the remote table
    pub fn subscribe(&mut self) -> Option<()> {
        self.send(
            self.resolved_ids.mthd_subscribe?,
            vec![],
            Some(Self::on_subscribe_reply),
        )
    }

    /// Ask to insert data into the remote table
    pub fn ask_insert(&mut self, rows: Vec<Vec<Value>>) -> Option<()> {
        self.send(self.resolved_ids.mthd_insert?, vec![to_cbor(&rows)], None)
    }

    /// Ask to update keys on the remote table
    pub fn ask_update(
        &mut self,
        keys: Vec<i64>,
        rows: Vec<Vec<Value>>,
    ) -> Option<()> {
        self.send(
            self.resolved_ids.mthd_update?,
            vec![to_cbor(&keys), to_cbor(&rows)],
            None,
        )
    }

    /// Ask to remove keys from the remote table
    pub fn ask_remove(&mut self, keys: Vec<i64>) -> Option<()> {
        self.send(self.resolved_ids.mthd_remove?, vec![to_cbor(&keys)], None)
    }

    /// Ask to clear the table
    pub fn ask_clear(&mut self) -> Option<()> {
        self.send(self.resolved_ids.mthd_clear?, vec![], None)
    }

    /// Ask to update a specific selection
    pub fn ask_update_selection(&mut self, selection: Selection) -> Option<()> {
        self.send(
            self.resolved_ids.mthd_update_selection?,
            vec![to_cbor(&selection)],
            None,
        )
    }

    fn send(
        &mut self,
        method: NooID,
        args: Vec<Value>,
        on_done: Option<MethodHandler<T>>,
    ) -> Option<()> {
        let mut msg = ClientInvokeMessage {
            method,
            context: Some(InvokeIDType::Table(self.remote_table_id)),
            invoke_id: None,
            args,
        };

        if let Some(h) = on_done {
            let reply_id = uuid::Uuid::new_v4();
            msg.invoke_id = Some(reply_id.to_string());

            self.methods_in_flight.insert(reply_id, h);
        }

        self.to_server
            .blocking_send(OutgoingMessage::MethodInvoke(msg))
            .ok()
    }

    /// Check an incoming method reply to see if it is relevant to this managed table
    pub fn check_relevant_method(&self, method: &MessageMethodReply) -> bool {
        if let Ok(id) = method.invoke_id.parse() {
            return self.methods_in_flight.contains_key(&id);
        }
        false
    }

    /// if [check_relevant_method] returns true, consume the message
    pub fn consume_relevant_method(&mut self, method: MessageMethodReply) {
        if let Ok(id) = method.invoke_id.parse() {
            if let Some(func) = self.methods_in_flight.get(&id) {
                (func)(self, method);
                self.methods_in_flight.remove(&id);
            }
        }
    }

    /// Check an incoming signal to see if it is relevant to this managed table
    pub fn check_relevant_signal(
        &mut self,
        signal: &ClientMessageSignalInvoke,
    ) -> bool {
        if let Some(id) = self.resolved_ids.sig_reset {
            if id == signal.id {
                return true;
            }
        }

        if let Some(id) = self.resolved_ids.sig_row_remove {
            if id == signal.id {
                return true;
            }
        }

        if let Some(id) = self.resolved_ids.sig_updated {
            if id == signal.id {
                return true;
            }
        }

        if let Some(id) = self.resolved_ids.sig_selection_update {
            if id == signal.id {
                return true;
            }
        }

        false
    }

    /// If [check_relevant_signal] is true, consume the signal
    pub fn consume_relevant_signal(
        &mut self,
        signal: ClientMessageSignalInvoke,
    ) -> Option<()> {
        if let Some(id) = self.resolved_ids.sig_reset {
            if id == signal.id {
                self.table_data.clear();
                return Some(());
            }
        }

        if let Some(id) = self.resolved_ids.sig_row_remove {
            if id == signal.id {
                let keys = arg_to_tuple!(signal.signal_data, Vec<i64>)?;
                self.table_data.delete_data(keys.0);
                return Some(());
            }
        }

        if let Some(id) = self.resolved_ids.sig_updated {
            if id == signal.id {
                let (keys, rows) = arg_to_tuple!(
                    signal.signal_data,
                    Vec<i64>,
                    Vec<Vec<Value>>
                )?;
                self.table_data.update_data(keys, rows);
                return Some(());
            }
        }

        if let Some(id) = self.resolved_ids.sig_selection_update {
            if id == signal.id {
                let selection = arg_to_tuple!(signal.signal_data, Selection)?;
                self.table_data.update_selection(selection.0);
                return Some(());
            }
        }

        None
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
