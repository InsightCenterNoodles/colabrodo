use std::collections::HashMap;

use colabrodo_common::{
    arg_to_tuple,
    client_communication::{ClientInvokeMessage, InvokeIDType},
    common::strings,
    nooid::NooID,
    table::*,
    value_tools::*,
};

use crate::{
    client::{ComponentList, OutgoingMessage, UserClientState},
    components::ClientMessageSignalInvoke,
};

pub trait TableTrait {
    fn on_init_data(&self, data: TableInitData);

    fn update_data(&mut self, keys: Vec<i64>, updated_data: Vec<Vec<Value>>);

    fn delete_data(&mut self, keys: Vec<i64>);

    fn update_selection(&mut self, selection: Selection);

    fn clear(&mut self);
}

// =============================================================================

struct SignalInfo {
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

impl SignalInfo {
    fn new(state: &mut impl UserClientState) -> Self {
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

type MethodHandler<T> = fn(&mut ManagedTable<T>);

pub struct ManagedTable<T: TableTrait> {
    remote_table_id: NooID,
    table_data: T,
    to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    resolved_ids: SignalInfo,
    methods_in_flight: HashMap<uuid::Uuid, MethodHandler<T>>,
}

impl<T: TableTrait> ManagedTable<T> {
    pub fn new(
        remote_table_id: NooID,
        state: &mut impl UserClientState,
        table_data: T,
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        let resolved_ids = SignalInfo::new(state);

        Self {
            remote_table_id,
            table_data,
            to_server,
            resolved_ids,
            methods_in_flight: HashMap::new(),
        }
    }

    pub fn on_subscribe_reply(&mut self) {}

    pub fn subscribe(&mut self) -> Option<()> {
        self.send(
            self.resolved_ids.mthd_subscribe?,
            vec![],
            Some(Self::on_subscribe_reply),
        )
    }

    pub fn ask_insert(&mut self, rows: Vec<Vec<Value>>) -> Option<()> {
        self.send(self.resolved_ids.mthd_insert?, vec![to_cbor(&rows)], None)
    }

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

    pub fn ask_remove(&mut self, keys: Vec<i64>) -> Option<()> {
        self.send(self.resolved_ids.mthd_remove?, vec![to_cbor(&keys)], None)
    }

    pub fn ask_clear(&mut self) -> Option<()> {
        self.send(self.resolved_ids.mthd_clear?, vec![], None)
    }

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

    pub fn check_poll_signal(
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

    pub fn poll_signal(
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
