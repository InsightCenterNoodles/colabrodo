//! Tools to help subscribe to server-side tables
use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
};

use colabrodo_common::{
    arg_to_tuple, common::strings, nooid::*,
    server_communication::MessageMethodReply, value_tools::*,
};

pub use colabrodo_common::table::*;

use crate::{
    client::InvokeContext,
    client_state::ClientDelegateLists,
    components::ClientTableUpdate,
    delegate::{Delegate, UpdatableDelegate},
};
use crate::{client_state::ClientState, components::ClientTableState};

// =============================================================================

/// Interface for a client-side table
pub trait TableDataStorage: Send {
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

enum TableAction {
    Subreply,
}

/// A delegate with convenience functions for subscription, adding, deleting, and updating data.
pub struct AdvTableDelegate {
    table_id: TableID,
    table: Option<Box<dyn TableDataStorage>>,
    method_subs: HashMap<uuid::Uuid, TableAction>,
}

impl AdvTableDelegate {
    pub fn new(id: TableID) -> Self {
        Self {
            table_id: id,
            table: Default::default(),
            method_subs: Default::default(),
        }
    }

    /// Subscribe to this table
    ///
    /// A reference to the client state is required to invoke methods. The function also needs a [TableDataStorage] that will be used to store data
    pub fn subscribe(
        &mut self,
        client: &mut ClientState,
        table: Box<dyn TableDataStorage>,
    ) {
        self.table = Some(table);

        let invoke_id = client.invoke_method(
            mthd_subscribe(&client.delegate_lists)
                .expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            Default::default(),
        );

        self.method_subs.insert(invoke_id, TableAction::Subreply);
    }

    fn on_sub_reply(&mut self, reply: MessageMethodReply) -> Option<()> {
        let init_data = from_cbor(reply.result?)
            .unwrap_or_else(|_| TableInitData::default());
        if let Some(t) = &mut self.table {
            t.on_init_data(init_data);
        }
        Some(())
    }

    /// Ask to insert records into the table
    pub fn ask_insert(
        &mut self,
        client: &mut ClientState,
        rows: Vec<Vec<Value>>,
    ) {
        let _res = client.invoke_method(
            mthd_insert(&client.delegate_lists).expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            vec![to_cbor(&rows)],
        );
    }

    /// Ask to update keys on the remote table
    pub fn ask_update(
        &mut self,
        client: &mut ClientState,
        keys: Vec<i64>,
        rows: Vec<Vec<Value>>,
    ) {
        let _res = client.invoke_method(
            mthd_update(&client.delegate_lists).expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            vec![to_cbor(&keys), to_cbor(&rows)],
        );
    }

    /// Ask to remove keys on the remote table
    pub fn ask_remove(&mut self, client: &mut ClientState, keys: Vec<i64>) {
        let _res = client.invoke_method(
            mthd_remove(&client.delegate_lists).expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            vec![to_cbor(&keys)],
        );
    }

    /// Ask to clear keys on the remote table
    pub fn ask_clear(&mut self, client: &mut ClientState) {
        let _res = client.invoke_method(
            mthd_clear(&client.delegate_lists).expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            vec![],
        );
    }

    /// Ask to update a specific selection
    pub fn ask_update_selection(
        &mut self,
        client: &mut ClientState,
        selection: Selection,
    ) {
        let _res = client.invoke_method(
            mthd_update_selection(&client.delegate_lists)
                .expect("Unable to subscribe."),
            InvokeContext::Table(self.table_id),
            vec![to_cbor(&selection)],
        );
    }

    fn on_signal_fallible(
        &mut self,
        id: SignalID,
        client: &mut ClientDelegateLists,
        args: Vec<ciborium::value::Value>,
    ) -> Option<()> {
        // TODO: In the future we can just rescan ids on update.

        if let Some(tbl) = &mut self.table {
            let sig_reset_id = sig_reset(client)?;
            let sig_updated_id = sig_updated(client)?;
            let sig_remove_id = sig_row_remove(client)?;
            let sig_selection_update_id = sig_selection_update(client)?;

            if id == sig_reset_id {
                tbl.clear()
            } else if id == sig_updated_id {
                let (keys, rows) =
                    arg_to_tuple!(args, Vec<i64>, Vec<Vec<Value>>)?;
                tbl.update_data(keys, rows);
            } else if id == sig_remove_id {
                let keys = arg_to_tuple!(args, Vec<i64>)?;
                tbl.delete_data(keys.0);
            } else if id == sig_selection_update_id {
                let selection = arg_to_tuple!(args, Selection)?;
                tbl.update_selection(selection.0);
            } else {
                // we do nothing.
            }
        }
        Some(())
    }
}

// =============================================================================

impl Delegate for AdvTableDelegate {
    type IDType = TableID;
    type InitStateType = ClientTableState;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl UpdatableDelegate for AdvTableDelegate {
    type UpdateStateType = ClientTableUpdate;

    fn on_method_reply(
        &mut self,
        _client: &mut ClientDelegateLists,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
        if let Some(action) = self.method_subs.remove(&invoke_id) {
            match action {
                TableAction::Subreply => {
                    self.on_sub_reply(reply);
                }
            }
        }
    }

    fn on_signal(
        &mut self,
        id: SignalID,
        client: &mut ClientDelegateLists,
        args: Vec<ciborium::value::Value>,
    ) {
        self.on_signal_fallible(id, client, args);
    }
}

// =============================================================================

pub fn sig_reset(state: &ClientDelegateLists) -> Option<SignalID> {
    state.signal_list.get_id_by_name(strings::SIG_TBL_RESET)
}

pub fn sig_updated(state: &ClientDelegateLists) -> Option<SignalID> {
    state.signal_list.get_id_by_name(strings::SIG_TBL_UPDATED)
}

pub fn sig_row_remove(state: &ClientDelegateLists) -> Option<SignalID> {
    state
        .signal_list
        .get_id_by_name(strings::SIG_TBL_ROWS_REMOVED)
}

pub fn sig_selection_update(state: &ClientDelegateLists) -> Option<SignalID> {
    state
        .signal_list
        .get_id_by_name(strings::SIG_TBL_SELECTION_UPDATED)
}

pub fn mthd_subscribe(state: &ClientDelegateLists) -> Option<MethodID> {
    state
        .method_list
        .get_id_by_name(strings::MTHD_TBL_SUBSCRIBE)
}

pub fn mthd_insert(state: &ClientDelegateLists) -> Option<MethodID> {
    state.method_list.get_id_by_name(strings::MTHD_TBL_INSERT)
}

pub fn mthd_update(state: &ClientDelegateLists) -> Option<MethodID> {
    state.method_list.get_id_by_name(strings::MTHD_TBL_UPDATE)
}

pub fn mthd_remove(state: &ClientDelegateLists) -> Option<MethodID> {
    state.method_list.get_id_by_name(strings::MTHD_TBL_REMOVE)
}

pub fn mthd_clear(state: &ClientDelegateLists) -> Option<MethodID> {
    state.method_list.get_id_by_name(strings::MTHD_TBL_CLEAR)
}

pub fn mthd_update_selection(state: &ClientDelegateLists) -> Option<MethodID> {
    state
        .method_list
        .get_id_by_name(strings::MTHD_TBL_UPDATE_SELECTION)
}

// =============================================================================

/// A simple data storage class for use with managed tables.
///
/// This class can be used directly, or used as a model for custom user classes
#[derive(Debug, Default)]
pub struct BasicTable {
    /// List of table columns
    header: Vec<TableColumnInfo>,

    /// A not-so-optimal storage for table data.
    /// This is row oriented
    data: BTreeMap<i64, Vec<Value>>,

    /// Storage for table selection
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
