use std::collections::{BTreeMap, HashMap};

use crate::{
    server::ClientRecord,
    server_messages::{ComponentReference, Recorder, ServerTableState},
    server_state::{
        ServerMessageSignalInvoke, ServerSignalInvokeObj, ServerState,
    },
};
use ciborium::value::Value;
use colabrodo_common::{
    arg_to_tuple,
    client_communication::ClientInvokeMessage,
    common::strings,
    components::{MethodArg, MethodState, SignalState},
    server_communication::ServerMessageID,
    table::{Selection, TableColumnInfo, TableInitData},
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
}

impl TableSystemInit {
    pub fn new(state: &mut ServerState) -> Self {
        Self {
            sig_reset: state.signals.new_component(SignalState {
                name: strings::SIG_TBL_RESET.to_string(),
                doc: Some("The table context has been reset.".to_string()),
                arg_doc: vec![MethodArg {
                    name: "table_init".to_string(),
                    doc: Some("New table data".to_string()),
                }],
            }),
            sig_updated: state.signals.new_component(SignalState {
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
            }),
            sig_row_remove: state.signals.new_component(SignalState {
                name: strings::SIG_TBL_ROWS_REMOVED.to_string(),
                doc: Some("Rows have been removed from the table".to_string()),
                arg_doc: vec![MethodArg {
                    name: "key_list".to_string(),
                    doc: Some("Keys to remove from the table".to_string()),
                }],
            }),
            sig_selection_update: state.signals.new_component(SignalState {
                name: strings::SIG_TBL_SELECTION_UPDATED.to_string(),
                doc: Some(
                    "A selection for the table has been updated".to_string(),
                ),
                arg_doc: vec![MethodArg {
                    name: "selection".to_string(),
                    doc: Some("A Selection type".to_string()),
                }],
            }),
            mthd_subscribe: state.methods.new_component(MethodState {
                name: strings::MTHD_TBL_SUBSCRIBE.to_string(),
                doc: Some("Subscribe to the given table".to_string()),
                return_doc: Some("Initial table data structure".to_string()),
                arg_doc: vec![],
            }),
            mthd_insert: state.methods.new_component(MethodState {
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
            }),
            mthd_update: state.methods.new_component(MethodState {
                name: strings::MTHD_TBL_UPDATE.to_string(),
                doc: Some("Ask to update data in the table".to_string()),
                return_doc: None,
                arg_doc: vec![
                    MethodArg {
                        name: "keys".to_string(),
                        doc: Some(
                            "A list of keys, one for each row."
                                .to_string(),
                        ),
                    },
                    MethodArg {
                        name: "rows".to_string(),
                        doc: Some(
                            "A list of rows, which is a list of data to update."
                                .to_string(),
                        ),
                    }
                ],
            }),
            mthd_remove: state.methods.new_component(MethodState {
                name: strings::MTHD_TBL_REMOVE.to_string(),
                doc: Some("Ask to remove data in the table".to_string()),
                return_doc: None,
                arg_doc: vec![MethodArg {
                    name: "keys".to_string(),
                    doc: Some("A list of keys to remove.".to_string()),
                }],
            }),
            mthd_clear: state.methods.new_component(MethodState {
                name: strings::MTHD_TBL_CLEAR.to_string(),
                doc: Some("Ask to clear all data in the table".to_string()),
                return_doc: None,
                arg_doc: vec![],
            }),
            mthd_update_selection: state.methods.new_component(MethodState {
                name: strings::MTHD_TBL_UPDATE_SELECTION.to_string(),
                doc: Some("Ask to update a selection in the table".to_string()),
                return_doc: None,
                arg_doc: vec![MethodArg {
                    name: "selection".to_string(),
                    doc: Some("A selection object".to_string()),
                }],
            }),
        }
    }

    pub fn is_table_message(&self, msg: &ClientInvokeMessage) -> bool {
        [
            self.mthd_subscribe.id(),
            self.mthd_insert.id(),
            self.mthd_update.id(),
            self.mthd_remove.id(),
            self.mthd_clear.id(),
            self.mthd_update_selection.id(),
        ]
        .into_iter()
        .find(|x| *x == msg.method)
        .is_some()
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
        init: TableSystemInit,
        table_id: ComponentReference<ServerTableState>,
        table: T,
    ) -> Self {
        Self {
            init_info: init,
            table_id,
            table_type: table,
            subscribers: Default::default(),
        }
    }

    pub fn subscribe(
        &mut self,
        id: uuid::Uuid,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> TableInitData {
        self.subscribers.insert(id, sender);
        self.table_type.get_init_data()
    }

    pub fn forget_client(&mut self, id: uuid::Uuid) {
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

    pub fn is_table_message(&self, msg: &ClientInvokeMessage) -> bool {
        self.init_info.is_table_message(msg)
    }

    pub fn handle_next(
        &mut self,
        cr: &ClientRecord,
        msg: ClientInvokeMessage,
    ) -> Option<()> {
        if self.init_info.mthd_subscribe.id() == msg.method {
            self.subscribe(cr.id, cr.sender.clone());
        } else if self.init_info.mthd_insert.id() == msg.method {
            self.insert(from_cbor_list(msg.args).ok()?)?
        } else if self.init_info.mthd_update.id() == msg.method {
            let (keys, rows) =
                arg_to_tuple!(msg.args, Vec<i64>, Vec<Vec<Value>>)?;
            self.update(keys, rows)?;
        } else if self.init_info.mthd_remove.id() == msg.method {
            let keys = arg_to_tuple!(msg.args, Vec<i64>)?;
            self.remove(keys.0);
        } else if self.init_info.mthd_clear.id() == msg.method {
            self.clear()?;
        } else if self.init_info.mthd_update_selection.id() == msg.method {
            let args = arg_to_tuple!(msg.args, Selection)?;
            self.update_selection(args.0)?;
        }

        Some(())
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

pub struct BasicTable {
    header: Vec<TableColumnInfo>,
    data: BTreeMap<i64, Vec<Value>>,
    selections: HashMap<String, Selection>,
    counter: i64,
}

impl BasicTable {
    pub fn new(header: Vec<TableColumnInfo>) -> Self {
        Self {
            header,
            data: BTreeMap::new(),
            selections: HashMap::new(),
            counter: 0,
        }
    }

    fn next_key(&mut self) -> i64 {
        let now = self.counter;
        self.counter += 1;
        return now;
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

        for l in new_data {
            if l.len() != self.header.len() {
                continue;
            }
            fixed_data.push(l);
            new_keys.push(self.next_key());
        }

        if new_keys.is_empty() {
            return None;
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
            if let Some(_) = self.data.remove(&k) {
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
