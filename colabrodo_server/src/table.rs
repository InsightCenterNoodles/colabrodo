use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    server::ClientRecord,
    server_messages::{
        ComponentReference, Recorder, ServerTableState,
        ServerTableStateUpdatable, UpdatableStateItem,
    },
    server_state::{
        InvokeObj, MethodResult, ServerMessageSignalInvoke,
        ServerSignalInvokeObj, ServerState,
    },
};
use ciborium::value::Value;
pub use colabrodo_common::table::{Selection, TableColumnInfo, TableInitData};
use colabrodo_common::{
    arg_to_tuple,
    common::strings,
    components::{MethodArg, MethodState, SignalState},
    server_communication::{ExceptionCodes, MethodException, ServerMessageID},
    tf_to_cbor,
    value_tools::*,
};
use tokio::sync::mpsc;

// =============================================================================

#[derive(Debug, Clone)]
pub struct TableSystemInit {
    sig_reset: ComponentReference<SignalState>,
    sig_updated: ComponentReference<SignalState>,
    sig_row_remove: ComponentReference<SignalState>,
    sig_selection_update: ComponentReference<SignalState>,

    mthd_subscribe: ComponentReference<MethodState>,
    mthd_insert: ComponentReference<MethodState>,
    mthd_update: ComponentReference<MethodState>,

    mthd_remove: ComponentReference<MethodState>,
    mthd_clear: ComponentReference<MethodState>,
    mthd_update_selection: ComponentReference<MethodState>,

    valid_signal_hash: HashSet<ComponentReference<SignalState>>,
    valid_method_hash: HashSet<ComponentReference<MethodState>>,
}

impl TableSystemInit {
    pub fn new(state: &mut ServerState) -> Self {
        let mthd_subscribe = state.methods.new_component(MethodState {
            name: strings::MTHD_TBL_SUBSCRIBE.to_string(),
            doc: Some("Subscribe to the given table".to_string()),
            return_doc: Some("Initial table data structure".to_string()),
            arg_doc: vec![],
        });
        let mthd_insert = state.methods.new_component(MethodState {
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
        });
        let mthd_update = state.methods.new_component(MethodState {
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
        });
        let mthd_remove = state.methods.new_component(MethodState {
            name: strings::MTHD_TBL_REMOVE.to_string(),
            doc: Some("Ask to remove data in the table".to_string()),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "keys".to_string(),
                doc: Some("A list of keys to remove.".to_string()),
            }],
        });
        let mthd_clear = state.methods.new_component(MethodState {
            name: strings::MTHD_TBL_CLEAR.to_string(),
            doc: Some("Ask to clear all data in the table".to_string()),
            return_doc: None,
            arg_doc: vec![],
        });
        let mthd_update_selection = state.methods.new_component(MethodState {
            name: strings::MTHD_TBL_UPDATE_SELECTION.to_string(),
            doc: Some("Ask to update a selection in the table".to_string()),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "selection".to_string(),
                doc: Some("A selection object".to_string()),
            }],
        });

        let mut method_set = HashSet::new();
        method_set.insert(mthd_subscribe.clone());
        method_set.insert(mthd_insert.clone());
        method_set.insert(mthd_update.clone());
        method_set.insert(mthd_remove.clone());
        method_set.insert(mthd_clear.clone());
        method_set.insert(mthd_update_selection.clone());

        let sig_reset = state.signals.new_component(SignalState {
            name: strings::SIG_TBL_RESET.to_string(),
            doc: Some("The table context has been reset.".to_string()),
            arg_doc: vec![MethodArg {
                name: "table_init".to_string(),
                doc: Some("New table data".to_string()),
            }],
        });
        let sig_updated = state.signals.new_component(SignalState {
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
        });
        let sig_row_remove = state.signals.new_component(SignalState {
            name: strings::SIG_TBL_ROWS_REMOVED.to_string(),
            doc: Some("Rows have been removed from the table".to_string()),
            arg_doc: vec![MethodArg {
                name: "key_list".to_string(),
                doc: Some("Keys to remove from the table".to_string()),
            }],
        });
        let sig_selection_update = state.signals.new_component(SignalState {
            name: strings::SIG_TBL_SELECTION_UPDATED.to_string(),
            doc: Some("A selection for the table has been updated".to_string()),
            arg_doc: vec![MethodArg {
                name: "selection".to_string(),
                doc: Some("A Selection type".to_string()),
            }],
        });

        let mut signal_set = HashSet::new();
        signal_set.insert(sig_reset.clone());
        signal_set.insert(sig_updated.clone());
        signal_set.insert(sig_row_remove.clone());
        signal_set.insert(sig_selection_update.clone());

        Self {
            sig_reset,
            sig_updated,
            sig_row_remove,
            sig_selection_update,

            mthd_subscribe,
            mthd_insert,
            mthd_update,
            mthd_remove,
            mthd_clear,
            mthd_update_selection,

            valid_signal_hash: signal_set,
            valid_method_hash: method_set,
        }
    }

    fn attach(
        &self,
        state: &mut ServerState,
        table: &ComponentReference<ServerTableState>,
    ) {
        let methods_to_update = state
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

        let signals_to_update = state
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

        let mut update = ServerTableStateUpdatable::default();

        update.methods_list = Some(methods_to_update);
        update.signals_list = Some(signals_to_update);

        update.patch(table);
    }

    fn is_table_message(
        &self,
        method: &ComponentReference<MethodState>,
    ) -> bool {
        let ret = self.valid_method_hash.contains(method);
        log::debug!("Checking if method is a table method: {ret}");
        ret
    }
}

// =============================================================================

pub trait TableTrait {
    fn get_init_data(&self) -> TableInitData;

    fn insert_data(
        &mut self,
        new_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)>;

    fn update_data(
        &mut self,
        keys: Vec<i64>,
        updated_data: Vec<Vec<Value>>,
    ) -> Option<(Vec<i64>, Vec<Vec<Value>>)>;

    fn delete_data(&mut self, keys: Vec<i64>) -> Option<Vec<i64>>;

    fn update_selection(&mut self, selection: Selection) -> Option<Selection>;

    fn clear(&mut self) -> Option<TableInitData>;
}

// =============================================================================

pub struct TableStore<T: TableTrait> {
    init_info: TableSystemInit,
    table_id: ComponentReference<ServerTableState>,
    table_type: T,
    subscribers: HashMap<uuid::Uuid, mpsc::Sender<Vec<u8>>>,
}

impl<T: TableTrait> TableStore<T> {
    pub fn new(
        state: &mut ServerState,
        init: TableSystemInit,
        table_id: ComponentReference<ServerTableState>,
        table: T,
    ) -> Self {
        init.attach(state, &table_id);
        Self {
            init_info: init,
            table_id,
            table_type: table,
            subscribers: Default::default(),
        }
    }

    fn subscribe(
        &mut self,
        id: uuid::Uuid,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> TableInitData {
        log::debug!("Subscribing {id}");
        self.subscribers.insert(id, sender);
        self.table_type.get_init_data()
    }

    pub fn forget_client(&mut self, id: uuid::Uuid) {
        log::debug!("Forgetting {id}");
        self.subscribers.remove(&id);
    }

    pub fn insert(&mut self, new_data: Vec<Vec<Value>>) -> Option<()> {
        let (keys, fixed) = self.table_type.insert_data(new_data)?;

        let args = tf_to_cbor![keys, fixed];

        self.broadcast(&self.init_info.sig_updated, args);

        Some(())
    }

    pub fn update(
        &mut self,
        keys: Vec<i64>,
        rows: Vec<Vec<Value>>,
    ) -> Option<()> {
        let (keys, fixed) = self.table_type.update_data(keys, rows)?;

        let args = tf_to_cbor![keys, fixed];

        self.broadcast(&self.init_info.sig_updated, args);

        Some(())
    }

    pub fn remove(&mut self, keys: Vec<i64>) -> Option<()> {
        let keys = self.table_type.delete_data(keys)?;

        let args = tf_to_cbor![keys];

        self.broadcast(&self.init_info.sig_row_remove, args);

        Some(())
    }

    pub fn update_selection(&mut self, selection: Selection) -> Option<()> {
        let sel = self.table_type.update_selection(selection)?;

        let args = tf_to_cbor![sel];

        self.broadcast(&self.init_info.sig_selection_update, args);

        Some(())
    }

    pub fn clear(&mut self) -> Option<()> {
        let new_state = self.table_type.clear()?;

        self.broadcast(&self.init_info.sig_reset, tf_to_cbor![new_state]);

        Some(())
    }

    pub fn can_handle_next(
        &self,
        method: &ComponentReference<MethodState>,
        context: &InvokeObj,
    ) -> bool {
        if let InvokeObj::Table(t) = context {
            return (t.id() == self.table_id.id())
                && self.init_info.is_table_message(method);
        }
        return false;
    }

    pub fn handle_next(
        &mut self,
        cr: &ClientRecord,
        method: &ComponentReference<MethodState>,
        context: InvokeObj,
        args: Vec<Value>,
    ) -> MethodResult {
        log::debug!("Handle next table command: {method:?}");
        if !{
            if let InvokeObj::Table(t) = context {
                (t == self.table_id) && self.init_info.is_table_message(method)
            } else {
                log::debug!("Context is not a table.");
                false
            }
        } {
            return Err(MethodException::internal_error(Some(
                "Given a bad method to invoke on a table.".to_string(),
            )));
        }

        log::debug!("Table handler: Client {}", cr.id);

        fn translate_return(r: Option<()>) -> MethodResult {
            if r.is_some() {
                return Ok(None);
            }
            return Err(MethodException {
                code: ExceptionCodes::InvalidParameters as i32,
                ..Default::default()
            });
        }

        fn map_bad_args() -> MethodException {
            return MethodException {
                code: ExceptionCodes::InvalidParameters as i32,
                ..Default::default()
            };
        }

        if self.init_info.mthd_subscribe == *method {
            let init = self.subscribe(cr.id, cr.sender.clone());
            return Ok(Some(to_cbor(&init)));
        } else if self.init_info.mthd_insert == *method {
            return translate_return(
                self.insert(from_cbor_list(args).map_err(|_| map_bad_args())?),
            );
        } else if self.init_info.mthd_update == *method {
            let (keys, rows) = arg_to_tuple!(args, Vec<i64>, Vec<Vec<Value>>)
                .ok_or_else(map_bad_args)?;
            return translate_return(self.update(keys, rows));
        } else if self.init_info.mthd_remove == *method {
            let keys =
                arg_to_tuple!(args, Vec<i64>).ok_or_else(map_bad_args)?;
            return translate_return(self.remove(keys.0));
        } else if self.init_info.mthd_clear == *method {
            return translate_return(self.clear());
        } else if self.init_info.mthd_update_selection == *method {
            let args =
                arg_to_tuple!(args, Selection).ok_or_else(map_bad_args)?;
            return translate_return(self.update_selection(args.0));
        }

        log::error!("Missing table method invoke");

        return Err(MethodException::internal_error(None));
    }

    fn broadcast(
        &self,
        signal: &ComponentReference<SignalState>,
        args: Vec<Value>,
    ) {
        let data = {
            let sig_info = ServerMessageSignalInvoke {
                id: signal.id(),
                context: Some(ServerSignalInvokeObj {
                    table: Some(self.table_id.clone()),
                    ..Default::default()
                }),
                signal_data: args,
            };

            Recorder::record(ServerMessageSignalInvoke::message_id(), &sig_info)
        };

        for client in self.subscribers.values() {
            client.blocking_send(data.data.clone()).unwrap();
        }
    }
}

// =============================================================================

#[derive(Debug)]
pub struct BasicTable {
    header: Vec<TableColumnInfo>,
    data: BTreeMap<i64, Vec<Value>>,
    selections: HashMap<String, Selection>,
    counter: i64,
}

impl BasicTable {
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
