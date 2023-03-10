use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, Mutex},
};

use ciborium::value::Value;
use colabrodo_common::{
    arg_to_tuple,
    client_communication::InvokeIDType,
    common::strings,
    server_communication::{MethodException, ServerMessageID},
    tf_to_cbor,
    value_tools::*,
};
use tokio::sync::mpsc;

pub use colabrodo_common::table::*;

use closure::closure;

use crate::{
    server_messages::*,
    server_state::{
        MethodResult, ServerMessageSignalInvoke, ServerSignalInvokeObj,
        ServerState,
    },
};

// =============================================================================

#[derive(Clone)]
pub struct TableSignals {
    sig_reset: SignalReference,
    sig_updated: SignalReference,
    sig_row_remove: SignalReference,
    sig_selection_update: SignalReference,
}

impl TableSignals {
    fn new(state: &mut Arc<Mutex<ServerState>>) -> Self {
        let mut lock = state.lock().unwrap();

        let sig_reset = lock.signals.new_component(ServerSignalState {
            name: strings::SIG_TBL_RESET.to_string(),
            doc: Some("The table context has been reset.".to_string()),
            arg_doc: vec![MethodArg {
                name: "table_init".to_string(),
                doc: Some("New table data".to_string()),
            }],
            ..Default::default()
        });
        let sig_updated = lock.signals.new_component(ServerSignalState {
            name: strings::SIG_TBL_UPDATED.to_string(),
            doc: Some("The table rows have been updated".to_string()),
            arg_doc: vec![
                MethodArg {
                    name: "key_list".to_string(),
                    doc: Some("Keys for new/updated rows".to_string()),
                },
                MethodArg {
                    name: "row_list".to_string(),
                    doc: Some("New/updated rows".to_string()),
                },
            ],
            ..Default::default()
        });
        let sig_row_remove = lock.signals.new_component(ServerSignalState {
            name: strings::SIG_TBL_ROWS_REMOVED.to_string(),
            doc: Some("Rows have been removed from the table".to_string()),
            arg_doc: vec![MethodArg {
                name: "key_list".to_string(),
                doc: Some("Keys to remove from the table".to_string()),
            }],
            ..Default::default()
        });
        let sig_selection_update =
            lock.signals.new_component(ServerSignalState {
                name: strings::SIG_TBL_SELECTION_UPDATED.to_string(),
                doc: Some(
                    "A selection for the table has been updated".to_string(),
                ),
                arg_doc: vec![MethodArg {
                    name: "selection".to_string(),
                    doc: Some("A Selection type".to_string()),
                }],
                ..Default::default()
            });

        Self {
            sig_reset,
            sig_updated,
            sig_row_remove,
            sig_selection_update,
        }
    }
}

// =============================================================================

pub type TableSystemPtr = Arc<Mutex<TableSystem>>;

pub struct TableSystem {
    state: std::sync::Weak<Mutex<ServerState>>,

    signals: TableSignals,

    managed_tables: HashMap<TableReference, Arc<Mutex<TableController>>>,

    valid_signal_hash: HashSet<SignalReference>,
    valid_method_hash: HashSet<MethodReference>,
}

impl TableSystem {
    pub fn new(mut state: Arc<Mutex<ServerState>>) -> Arc<Mutex<TableSystem>> {
        let ret = Arc::new(Mutex::new(TableSystem {
            state: Arc::downgrade(&state),
            signals: TableSignals::new(&mut state),
            managed_tables: HashMap::new(),
            valid_signal_hash: HashSet::new(),
            valid_method_hash: HashSet::new(),
        }));

        // init methods
        let mut lock = state.lock().unwrap();

        let tbl_sub = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_SUBSCRIBE.to_string(),
            doc: Some("Subscribe to the given table".to_string()),
            return_doc: Some("Initial table data structure".to_string()),
            arg_doc: vec![],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_subscribe(s, m, ret.clone())
                }),
            ),
        });

        let tbl_insert = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_INSERT.to_string(),
            doc: Some("Ask to insert data into the table".to_string()),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "rows".to_string(),
                doc: Some(
                    "A list of rows, which is a list of data to insert."
                        .to_string(),
                ),
            }],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_insert(s, m, ret.clone())
                }),
            ),
        });

        let tbl_update = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_UPDATE.to_string(),
            doc: Some("Ask to update data in the table".to_string()),
            return_doc: None,
            arg_doc: vec![
                MethodArg {
                    name: "keys".to_string(),
                    doc: Some("A list of keys, one for each row.".to_string()),
                },
                MethodArg {
                    name: "rows".to_string(),
                    doc: Some(
                        "A list of rows, which is a list of data to update."
                            .to_string(),
                    ),
                },
            ],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_update(s, m, ret.clone())
                }),
            ),
        });

        let tbl_remove = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_REMOVE.to_string(),
            doc: Some("Ask to remove data in the table".to_string()),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "keys".to_string(),
                doc: Some("A list of keys to remove.".to_string()),
            }],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_remove(s, m, ret.clone())
                }),
            ),
        });

        let tbl_clear = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_CLEAR.to_string(),
            doc: Some("Ask to clear all data in the table".to_string()),
            return_doc: None,
            arg_doc: vec![],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_clear(s, m, ret.clone())
                }),
            ),
        });

        let tbl_up_sel = lock.methods.new_owned_component(ServerMethodState {
            name: strings::MTHD_TBL_UPDATE_SELECTION.to_string(),
            doc: Some("Ask to update a selection in the table".to_string()),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "selection".to_string(),
                doc: Some("A selection object".to_string()),
            }],
            state: MethodHandlerSlot::new_from_closure(
                closure!(clone ret, |s, m| {
                   on_table_update_selection(s, m, ret.clone())
                }),
            ),
        });

        {
            let mut self_lock = ret.lock().unwrap();

            let marray = [
                tbl_clear, tbl_insert, tbl_remove, tbl_sub, tbl_up_sel,
                tbl_update,
            ];

            self_lock.valid_method_hash.extend(marray);

            let sarray = [
                self_lock.signals.sig_reset.clone(),
                self_lock.signals.sig_updated.clone(),
                self_lock.signals.sig_row_remove.clone(),
                self_lock.signals.sig_selection_update.clone(),
            ];

            self_lock.valid_signal_hash.extend(sarray);
        }

        ret
    }

    pub fn register_table(
        &mut self,
        table: TableReference,
        storage: impl TableTrait + 'static,
    ) {
        self.managed_tables.insert(
            table.clone(),
            Arc::new(Mutex::new(TableController {
                table_reference: table.clone(),
                signals: self.signals.clone(),
                subscribers: HashMap::new(),
                table: Box::new(storage),
            })),
        );

        if let Some(st) = self.state.upgrade() {
            self.attach(table, st);
        }
    }

    fn attach(
        &mut self,
        table: TableReference,
        state: Arc<Mutex<ServerState>>,
    ) {
        let lock = state.lock().unwrap();

        let methods_to_update = lock
            .tables
            .inspect(table.id(), |t| {
                let mut existing = HashSet::new();

                if let Some(l) = &t.mutable.methods_list {
                    existing.extend(l.clone());
                };

                existing.extend(self.valid_method_hash.iter().cloned());

                let ret: Vec<_> = existing.into_iter().collect();

                ret
            })
            .unwrap_or_else(|| {
                self.valid_method_hash.iter().cloned().collect()
            });

        let signals_to_update = lock
            .tables
            .inspect(table.id(), |t| {
                let mut existing = HashSet::new();

                if let Some(l) = &t.mutable.signals_list {
                    existing.extend(l.clone());
                };

                existing.extend(self.valid_signal_hash.iter().cloned());

                let ret: Vec<_> = existing.into_iter().collect();

                ret
            })
            .unwrap_or_else(|| {
                self.valid_signal_hash.iter().cloned().collect()
            });

        let update = ServerTableStateUpdatable {
            methods_list: Some(methods_to_update),
            signals_list: Some(signals_to_update),
            ..Default::default()
        };

        update.patch(&table);
    }
}

// =============================================================================

fn resolve_manager(
    state: &mut ServerState,
    context: Option<InvokeIDType>,
    table_system: Arc<Mutex<TableSystem>>,
) -> Result<Arc<Mutex<TableController>>, MethodException> {
    let table_id = context.and_then(|f| match f {
        InvokeIDType::Table(x) => Some(x),
        _ => None,
    });

    let table = table_id.and_then(|id| state.tables.resolve(id));

    let manager = table.and_then(|r| {
        table_system.lock().unwrap().managed_tables.get(&r).cloned()
    });

    manager.ok_or_else(|| MethodException::internal_error(None))
}

fn translate_result(ret: Option<()>) -> MethodResult {
    if ret.is_some() {
        Ok(None)
    } else {
        Err(MethodException::internal_error(None))
    }
}

fn on_table_subscribe(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let res = controller.on_table_subscribe(state, msg);

    Ok(Some(to_cbor(&res)))
}

fn on_table_insert(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let ret = controller.insert_data(
        from_cbor_list(msg.args)
            .map_err(|_| MethodException::invalid_parameters(None))?,
    );

    translate_result(ret)
}

fn on_table_update(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let (keys, data) = arg_to_tuple!(msg.args, Vec<i64>, Vec<Vec<Value>>)
        .ok_or_else(|| MethodException::invalid_parameters(None))?;

    let ret = controller.update(keys, data);

    translate_result(ret)
}

fn on_table_remove(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let (keys,) = arg_to_tuple!(msg.args, Vec<i64>)
        .ok_or_else(|| MethodException::invalid_parameters(None))?;

    let ret = controller.remove(keys);

    translate_result(ret)
}

fn on_table_update_selection(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let (selection,) = arg_to_tuple!(msg.args, Selection)
        .ok_or_else(|| MethodException::invalid_parameters(None))?;

    let ret = controller.update_selection(selection);

    translate_result(ret)
}

fn on_table_clear(
    state: &mut ServerState,
    msg: MethodSignalContent,
    table_system: Arc<Mutex<TableSystem>>,
) -> MethodResult {
    let controller = resolve_manager(state, msg.context, table_system)?;
    let mut controller = controller.lock().unwrap();

    let ret = controller.clear();

    translate_result(ret)
}

// =============================================================================

pub struct TableController {
    table_reference: TableReference,
    signals: TableSignals,
    subscribers: HashMap<uuid::Uuid, mpsc::UnboundedSender<Vec<u8>>>,
    table: Box<dyn TableTrait>,
}

impl TableController {
    fn on_table_subscribe(
        &mut self,
        state: &mut ServerState,
        msg: MethodSignalContent,
    ) -> TableInitData {
        self.subscribers.insert(
            msg.from,
            state.get_client_info(msg.from).unwrap().sender.clone(),
        );

        self.table.get_init_data()
    }

    fn broadcast(&self, signal: &SignalReference, args: Vec<Value>) {
        let data = {
            let sig_info = ServerMessageSignalInvoke {
                id: signal.id().0,
                context: Some(ServerSignalInvokeObj {
                    table: Some(self.table_reference.clone()),
                    ..Default::default()
                }),
                signal_data: args,
            };

            Recorder::record(ServerMessageSignalInvoke::message_id(), &sig_info)
        };

        for client in self.subscribers.values() {
            client.send(data.data.clone()).unwrap();
        }
    }

    pub fn insert_data(&mut self, new_data: Vec<Vec<Value>>) -> Option<()> {
        let (keys, fixed) = self.table.insert_data(new_data)?;

        let args = tf_to_cbor![keys, fixed];

        self.broadcast(&self.signals.sig_updated, args);

        Some(())
    }

    /// Update the table
    pub fn update(
        &mut self,
        keys: Vec<i64>,
        rows: Vec<Vec<Value>>,
    ) -> Option<()> {
        let (keys, fixed) = self.table.update_data(keys, rows)?;

        let args = tf_to_cbor![keys, fixed];

        self.broadcast(&self.signals.sig_updated, args);

        Some(())
    }

    /// Remove keys from the table
    pub fn remove(&mut self, keys: Vec<i64>) -> Option<()> {
        let keys = self.table.delete_data(keys)?;

        let args = tf_to_cbor![keys];

        self.broadcast(&self.signals.sig_row_remove, args);

        Some(())
    }

    /// Change a selection
    pub fn update_selection(&mut self, selection: Selection) -> Option<()> {
        let sel = self.table.update_selection(selection)?;

        let args = tf_to_cbor![sel];

        self.broadcast(&self.signals.sig_selection_update, args);

        Some(())
    }

    /// Clear the table
    pub fn clear(&mut self) -> Option<()> {
        let new_state = self.table.clear()?;

        self.broadcast(&self.signals.sig_reset, tf_to_cbor![new_state]);

        Some(())
    }
}

// =============================================================================

/// Interface for interacting with a table.
pub trait TableTrait {
    /// Called when a client needs initial data
    fn get_init_data(&self) -> TableInitData;

    /// Called when a client is asking to install data.
    ///
    /// This function should return None if the insert should fail. Otherwise, it should return keys for the newly inserted rows along with the data that was inserted. This allows your code to fix up inserted rows; either rejecting certain rows, adding values, etc.
    fn insert_data(
        &mut self,
        new_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)>;

    /// Called when a client is asking to update data.
    ///
    /// This function should return None if the update should fail. Otherwise, it should return keys for the updated rows along with the corresponding data that was changed. This allows your code to fix up updated rows; either rejecting certain rows, adding values, etc.
    fn update_data(
        &mut self,
        keys: Vec<i64>,
        updated_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)>;

    /// Called when a client is requesting to delete data.
    ///
    /// Should return None if the request should fail. Otherwise the function should return keys that should actually be deleted.
    fn delete_data(&mut self, keys: Vec<i64>) -> Option<Vec<i64>>;

    /// Called when a client is requesting to update a Selection
    ///
    /// Return None if the request should fail. Otherwise return the actual update to the Selection.
    fn update_selection(&mut self, selection: Selection) -> Option<Selection>;

    /// Called when a client is requesting to clear the table
    ///
    /// Return None if the request should fail. Otherwise return the new 'blank' state.
    fn clear(&mut self) -> Option<TableInitData>;
}

// =============================================================================

/// An example table to be used to present information to clients. This can be used directly, or used as a guide to create your own
#[derive(Debug)]
pub struct BasicTable {
    header: Vec<TableColumnInfo>,
    data: BTreeMap<i64, Vec<Value>>,
    selections: HashMap<String, Selection>,
    counter: i64,
}

impl BasicTable {
    /// Create a new table
    ///
    /// The column count in the header and the initial data should match...
    pub fn new(
        header: Vec<TableColumnInfo>,
        init_data: Vec<Vec<Value>>,
    ) -> Self {
        let mut ret = Self {
            header,
            data: BTreeMap::new(),
            selections: HashMap::new(),
            counter: 0,
        };

        ret.insert_data(init_data);

        log::debug!("Basic table ready {ret:?}");

        ret
    }

    fn next_key(&mut self) -> i64 {
        let now = self.counter;
        self.counter += 1;
        now
    }
}

impl TableTrait for BasicTable {
    fn get_init_data(&self) -> TableInitData {
        TableInitData {
            columns: self.header.clone(),
            keys: self.data.keys().cloned().collect(),
            data: self.data.values().cloned().collect(),
            ..Default::default()
        }
    }

    fn insert_data(
        &mut self,
        new_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)> {
        let mut fixed_data = Vec::<Vec<Value>>::new();
        let mut new_keys = Vec::<i64>::new();

        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Inserting data: {:?}", new_data);
        }

        for l in new_data {
            if l.len() != self.header.len() {
                log::debug!("Skipping insert, row is not the right length");
                continue;
            }
            fixed_data.push(l);
            new_keys.push(self.next_key());
        }

        if new_keys.is_empty() {
            return None;
        }

        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Keys: {new_keys:?}, Fixed: {fixed_data:?}",);
        }

        for (k, v) in new_keys.iter().zip(fixed_data.iter()) {
            self.data.insert(*k, v.clone());
        }

        Some((new_keys, fixed_data))
    }

    fn update_data(
        &mut self,
        keys: Vec<i64>,
        updated_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)> {
        if keys.len() != updated_data.len() {
            return None;
        }

        let mut out_keys = Vec::new();
        let mut out_data = Vec::new();

        for (k, v) in keys.into_iter().zip(updated_data.into_iter()) {
            self.data.entry(k).and_modify(|l| {
                *l = v.clone();
                out_keys.push(k);
                out_data.push(v);
            });
        }

        Some((out_keys, out_data))
    }

    fn delete_data(&mut self, keys: Vec<i64>) -> Option<Vec<i64>> {
        let mut ret = Vec::new();

        for k in keys {
            if self.data.remove(&k).is_some() {
                ret.push(k);
            }
        }

        Some(ret)
    }

    fn update_selection(&mut self, selection: Selection) -> Option<Selection> {
        self.selections
            .insert(selection.name.clone(), selection.clone());
        Some(selection)
    }

    fn clear(&mut self) -> Option<TableInitData> {
        self.data = BTreeMap::new();
        self.selections = HashMap::new();
        self.counter = 0;

        Some(self.get_init_data())
    }
}
