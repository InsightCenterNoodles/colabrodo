use colabrodo_common::recording::{self, parse_record, Packet, PacketStamp};
use colabrodo_server::server::ciborium;
use colabrodo_server::server::ciborium::value::Value;
use colabrodo_server::server_state::ServerState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::replay_client::{make_client_ptr, ReplayClientPtr};

pub struct ReplayerServerState {
    // we are going to just read everything in as is for now.
    packets: Vec<Packet>,
    marker_table: HashMap<String, recording::PacketStamp>,

    last_packet_index: u64,
    last_time: u64,

    client_state: ReplayClientPtr,
}

pub type ReplayerStatePtr = Arc<Mutex<ReplayerServerState>>;

impl ReplayerServerState {
    pub fn new(path: std::path::PathBuf) -> Arc<Mutex<Self>> {
        let in_file =
            std::fs::File::open(path).expect("Unable to open session file.");

        let in_stream = std::io::BufReader::new(in_file);

        // not the best, but for now...
        let raw_packet_vec: Vec<Value> = ciborium::de::from_reader(in_stream)
            .expect("Unable to decode record buffer");

        let packet_vec: Vec<_> = raw_packet_vec
            .into_iter()
            .filter_map(parse_record)
            .collect();

        let mut table: HashMap<String, PacketStamp> = HashMap::new();

        for packet in &packet_vec {
            match packet {
                Packet::DropMarker(time, name) => {
                    table.insert(name.clone(), *time)
                }
                Packet::WriteCBOR(_, _) => todo!(),
                Packet::BufferLocation(_) => todo!(),
            };
        }

        Arc::new_cyclic(|me| {
            Mutex::new(Self {
                packets: packet_vec,
                marker_table: table,
                last_packet_index: 0,
                last_time: 0,
                client_state: make_client_ptr(me.clone()),
            })
        })
    }

    fn advance(&mut self) -> Option<PacketStamp> {
        loop {
            let packet = self.packets.get(self.last_packet_index as usize)?;
            self.last_packet_index += 1;
            match packet {
                Packet::DropMarker(_, _) => continue,
                Packet::WriteCBOR(st, data) => {
                    crate::replay_client::advance_client(
                        &self.client_state,
                        data.as_slice(),
                    );
                    return Some(*st);
                }
                Packet::BufferLocation(_) => continue,
            }
        }
    }

    fn advance_to_time(&mut self, global_seconds: u64) -> Option<()> {
        loop {
            let next_time = self.advance()?.0 as u64;

            if next_time >= global_seconds {
                return Some(());
            }
        }
    }

    pub fn advance_by_time(
        &mut self,
        relative_seconds: u64,
        state: &mut ServerState,
    ) -> Option<()> {
        let target_ts = self.last_time + relative_seconds;

        self.advance_to_time(target_ts)
    }

    pub fn advance_by_marker(
        &mut self,
        label: String,
        state: &mut ServerState,
    ) -> Option<()> {
        let target_time = self.marker_table.get(&label)?.0 as u64;

        if target_time <= self.last_time {
            return None;
        }

        self.advance_to_time(target_time)
    }
}
