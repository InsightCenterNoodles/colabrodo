use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::process::abort;
use std::sync::{Arc, Mutex};

use colabrodo_client::mapped_client::ciborium::{de, ser, value};
use colabrodo_client::mapped_client::{
    self, handle_next, id_for_message, lookup, MappedNoodlesClient, NooValueMap,
};
use colabrodo_common::client_communication::{
    ClientIntroductionMessage, ClientInvokeMessage, ClientMessageID,
};
use colabrodo_common::common::{
    ComponentType, MessageArchType, ServerMessageIDs,
};

use colabrodo_common::nooid::NooID;
use futures_util::{future, pin_mut, SinkExt, StreamExt};

use tokio::runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use colabrodo_client::mapped_client::ciborium::value::Value;

use clap::Parser;

use env_logger;
use log::{debug, error, info, warn};

#[derive(Parser, Debug)]
struct CLIArgs {
    /// Server hostname
    url: String,

    /// Debug mode
    #[arg(short, long)]
    debug: bool,
}

type NooHashMap = HashMap<String, Value>;

/// Convert the linear representation of a dictionary to a mapped representation
fn convert(v: NooValueMap) -> NooHashMap {
    let mut ret: NooHashMap = NooHashMap::new();
    for (key, value) in v {
        let conv_key = key.as_text();
        if conv_key.is_none() {
            continue;
        }
        ret.insert(conv_key.unwrap().to_string(), value);
    }
    ret
}

/// A simple container for component data
#[derive(Default)]
struct ComponentList {
    components: HashMap<NooID, NooHashMap>,
}

impl ComponentList {
    fn insert(
        &mut self,
        tid: ComponentType,
        id: NooID,
        values: NooValueMap,
    ) -> Option<()> {
        let mut converted_map = convert(values);

        converted_map
            .entry("name".to_string())
            .or_insert(Value::Text(format!("{tid} {id}")));

        let ret = self.components.insert(id, converted_map);
        if ret.is_some() {
            // we clobbered a key
            warn!("Overwrote a component. This could be bad.")
        }

        Some(())
    }
    fn update(&mut self, id: NooID, values: NooValueMap) -> Option<()> {
        self.components.entry(id).and_modify(|v| {
            merge_values(values, v);
        });
        Some(())
    }
    fn delete(&mut self, id: NooID, _: NooValueMap) -> Option<()> {
        let ret = self.components.remove(&id);
        if ret.is_none() {
            error!("Asked to delete a component that does not exist!");
            return None;
        }
        Some(())
    }
    fn clear(&mut self) {
        self.components.clear();
    }

    fn find(&self, id: NooID) -> Option<&NooHashMap> {
        self.components.get(&id)
    }
}

pub fn merge_values(new_map: NooValueMap, dest: &mut NooHashMap) {
    for (k, v) in new_map {
        dest.insert(k.as_text().unwrap().to_string(), v);
    }
}

struct CLIState {
    world_list: Vec<ComponentList>,
    document: NooHashMap,

    methods_in_flight: HashMap<String, String>,
}

impl CLIState {
    fn new() -> CLIState {
        let list_count = 14; // replace with a feature to count enum values

        let mut clist = Vec::with_capacity(list_count);

        clist.resize_with(list_count, Default::default);

        CLIState {
            world_list: clist,
            document: NooHashMap::default(),
            methods_in_flight: HashMap::default(),
        }
    }

    fn clear(&mut self) {
        self.document.clear();
        for list in &mut self.world_list {
            list.clear();
        }
    }

    fn handle_component_message(
        &mut self,
        message: &ServerMessageIDs,
        content: NooValueMap,
    ) -> Option<()> {
        let index = message.component_type() as usize;

        let list = &mut self.world_list[index];

        let id = id_for_message(&content).unwrap();

        debug!("Message: {message} on {id}");

        let result = match message.arch_type() {
            MessageArchType::Create => {
                list.insert(message.component_type(), id, content)
            }
            MessageArchType::Update => list.update(id, content),
            MessageArchType::Delete => list.delete(id, content),
            MessageArchType::Other => panic!("Should not get here"),
        };

        if result.is_none() {
            debug!("Unable to handle message: {message} on {id}");
            abort()
        }

        result
    }

    fn handle_document_message(
        &mut self,
        message: &ServerMessageIDs,
        content: &NooValueMap,
    ) -> Option<()> {
        match message {
            ServerMessageIDs::MsgDocumentReset => {
                self.clear();
                Some(())
            }
            ServerMessageIDs::MsgDocumentUpdate => {
                merge_values(content.to_vec(), &mut self.document);
                Some(())
            }
            _ => Some(()),
        }
    }

    fn handle_special_message(
        &mut self,
        message: &ServerMessageIDs,
        content: &NooValueMap,
    ) -> Option<()> {
        match message {
            ServerMessageIDs::MsgSignalInvoke => {}
            ServerMessageIDs::MsgMethodReply => {
                self.handle_method_reply(content)?
            }
            ServerMessageIDs::MsgDocumentInitialized => {}
            _ => {}
        }

        Some(())
    }

    fn handle_method_reply(&mut self, content: &NooValueMap) -> Option<()> {
        let reply_id = lookup(&Value::Text("invoke_id".to_string()), content)?;
        let reply_id = reply_id.as_text()?.to_string();

        let reply = &self.methods_in_flight.get(&reply_id);

        if reply.is_none() {
            warn!("Reply for message we did not send: {content:?}")
        }

        self.methods_in_flight.remove(&reply_id);

        Some(())
    }
}

impl MappedNoodlesClient for CLIState {
    fn handle_message(
        &mut self,
        message: ServerMessageIDs,
        content: &NooValueMap,
    ) -> Result<(), mapped_client::UserError> {
        debug!("Message from server {}: {:?}", message, content);

        let ret = match message.component_type() {
            ComponentType::None => {
                self.handle_special_message(&message, content)
            }
            ComponentType::Document => {
                self.handle_document_message(&message, content)
            }
            _ => self.handle_component_message(&message, content.to_vec()),
        };

        if ret.is_none() {
            return Err(mapped_client::UserError::InternalError);
        }

        Ok(())
    }
}

async fn cli_main() {
    let cli_args = CLIArgs::parse();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    if cli_args.debug {
        env::set_var("RUST_LOG", "debug")
    }

    env_logger::init();

    let server = cli_args.url;

    let url = url::Url::parse(&server).unwrap();

    info!("Connecting to {}...", url);

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();

    let conn_result = connect_async(url).await;

    if conn_result.is_err() {
        error!("Unable to connect to given server.");
        abort();
    }

    let state: Arc<Mutex<CLIState>> = Arc::new(Mutex::new(CLIState::new()));

    tokio::spawn(read_stdin(stdin_tx, state.clone()));

    let (mut ws_stream, _) = conn_result.expect("Failed to connect");
    debug!("WebSocket handshake has been successfully completed");

    // send introduction
    {
        let introduction = (
            ClientIntroductionMessage::message_id(),
            ClientIntroductionMessage {
                client_name: "Rusty CLI".to_string(),
            },
        );

        let mut intro_bytes: Vec<u8> = Vec::new();

        ser::into_writer(&introduction, &mut intro_bytes)
            .expect("Unable to serialize introduction message!");

        ws_stream.send(Message::Binary(intro_bytes)).await.unwrap();
    }

    let (write, read) = ws_stream.split();

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);

    let state_clone = state.clone();

    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();

            let value: Value = de::from_reader(&data[..]).unwrap();

            {
                let mut state_ref = state_clone.lock().unwrap();

                handle_next(&value, &mut *state_ref).unwrap();
            }
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// =============================================================================

fn dump_list<Function>(source: &CLIState, ctype: ComponentType, extra: Function)
where
    Function: Fn(&NooID, &str, &NooHashMap),
{
    let target_list = &source.world_list[ctype as usize];
    for (k, v) in &target_list.components {
        extra(k, v["name"].as_text().unwrap_or("Unknown"), v);
    }
}

fn command_list(state: &Arc<Mutex<CLIState>>) {
    loop {
        let answer = requestty::prompt_one(
            requestty::Question::select("subcommand")
                .message("List what?")
                .choice("Entities")
                .choice("Tables")
                .choice("Plots")
                .choice("Materials")
                .choice("Geometry")
                .choice("Methods")
                .default_separator()
                .choice("Back"),
        )
        .unwrap();

        let answer = &answer.as_list_item().unwrap().text;

        // lock state

        let state_ref = state.lock().unwrap();

        match answer.as_str() {
            "Entities" => {
                dump_list(
                    &state_ref,
                    ComponentType::Entity,
                    |id: &NooID, name: &str, _: &NooHashMap| {
                        println!("{id} - {name}")
                    },
                );
            }
            "Tables" => {
                dump_list(
                    &state_ref,
                    ComponentType::Table,
                    |id: &NooID, name: &str, _: &NooHashMap| {
                        println!("{id} - {name}")
                    },
                );
            }
            "Plots" => {
                dump_list(
                    &state_ref,
                    ComponentType::Plot,
                    |id: &NooID, name: &str, _: &NooHashMap| {
                        println!("{id} - {name}")
                    },
                );
            }
            "Materials" => {
                dump_list(
                    &state_ref,
                    ComponentType::Material,
                    |id: &NooID, name: &str, _: &NooHashMap| {
                        println!("{id} - {name}")
                    },
                );
            }
            "Geometry" => {
                dump_list(
                    &state_ref,
                    ComponentType::Geometry,
                    |id: &NooID, name: &str, _: &NooHashMap| {
                        println!("{id} - {name}")
                    },
                );
            }
            "Methods" => {
                dump_list(
                    &state_ref,
                    ComponentType::Method,
                    |id: &NooID, name: &str, v: &NooHashMap| {
                        let none_str = Value::Text("None".to_string());
                        let doc = v
                            .get("doc")
                            .unwrap_or(&none_str)
                            .as_text()
                            .unwrap();
                        println!("{id} - {name} - {doc:?}");
                    },
                );
            }
            "Back" => return,
            _ => unreachable!(),
        }
    }
}

fn json_to_cbor(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                Value::Integer(value::Integer::from(n.as_i64().unwrap_or(0)))
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Text(s),
        serde_json::Value::Array(a) => {
            Value::Array(a.iter().map(|e| json_to_cbor(e.clone())).collect())
        }
        serde_json::Value::Object(o) => Value::Map(
            o.iter()
                .map(|e| {
                    (Value::Text(e.0.to_string()), json_to_cbor(e.1.clone()))
                })
                .collect(),
        ),
    }
}

fn collect_args(prompts: Vec<String>) -> Vec<Value> {
    let mut ret: Vec<Value> = Vec::new();

    for p in prompts {
        let opts = requestty::Question::input("argument").message(p).build();

        let code = requestty::prompt_one(opts).unwrap();

        let code = code.as_string();

        if code.is_none() {
            continue;
        }

        let code = code.unwrap();

        let json = serde_json::from_str(code).unwrap_or_default();

        ret.push(json_to_cbor(json));
    }

    ret
}

fn extract_method_arg_names(item: &NooHashMap) -> Option<Vec<String>> {
    // ick
    let name_name = Value::Text("name".to_string());

    let arg_doc = item.get("arg_doc")?;

    let arg_doc = match arg_doc {
        Value::Array(item) => item,
        _ => return None,
    };

    let mut ret: Vec<String> = Vec::new();

    for arg in arg_doc {
        let arg_info = match arg {
            Value::Map(map) => map,
            _ => continue,
        };

        let name_value = match lookup(&name_name, arg_info) {
            Some(value) => value,
            _ => continue,
        };

        let name_value = match name_value {
            Value::Text(t) => t,
            _ => continue,
        };

        ret.push(name_value.to_string());
    }

    Some(ret)
}

fn call_method(
    state: &Arc<Mutex<CLIState>>,
    tx: &futures_channel::mpsc::UnboundedSender<Message>,
) {
    loop {
        let mut opts =
            requestty::Question::select("subcommand").message("Call what?");

        let mut ids: Vec<NooID> = Vec::default();

        {
            let state_ref = state.lock().unwrap();

            let target_list =
                &state_ref.world_list[ComponentType::Method as usize];
            for (k, v) in &target_list.components {
                ids.push(*k);
                opts = opts.choice(v["name"].as_text().unwrap_or("Unknown"));
            }
        }

        opts = opts.default_separator();
        opts = opts.choice("Back");

        let answer = requestty::prompt_one(opts).unwrap();

        let answer = &answer.as_list_item().unwrap();

        if answer.text == "Back" {
            return;
        }

        let selected_id = ids[answer.index];

        let prompts: Vec<String>;

        {
            let state_ref = state.lock().unwrap();

            let target_list =
                &state_ref.world_list[ComponentType::Method as usize];

            let method_info = target_list.find(selected_id).unwrap();

            prompts = extract_method_arg_names(method_info).unwrap_or_default();
        }

        let invoke_identifier = uuid::Uuid::new_v4().to_string();

        let content = (
            ClientInvokeMessage::message_id(),
            ClientInvokeMessage {
                method: selected_id,
                invoke_id: Some(invoke_identifier.clone()),
                args: collect_args(prompts),
                ..Default::default()
            },
        );

        let mut buffer = Vec::<u8>::new();

        mapped_client::ciborium::ser::into_writer(&content, &mut buffer)
            .unwrap();

        // record this message

        {
            let mut state_ref = state.lock().unwrap();

            // TODO: replace with some kind of callback
            state_ref
                .methods_in_flight
                .insert(invoke_identifier, "called".to_string());
        }

        let send_result = tx.unbounded_send(Message::Binary(buffer));

        if send_result.is_err() {
            return;
        }
    }
}

async fn read_stdin(
    tx: futures_channel::mpsc::UnboundedSender<Message>,
    state: Arc<Mutex<CLIState>>,
) {
    //let mut stdin = tokio::io::stdin();

    loop {
        let answer = requestty::prompt_one(
            requestty::Question::select("command")
                .message("Command?")
                .choice("List")
                .choice("Call Method")
                .default_separator()
                .choice("Quit"),
        )
        .unwrap();

        match answer.as_list_item().unwrap().text.as_str() {
            "List" => command_list(&state),
            "Call Method" => call_method(&state, &tx),
            "Quit" => {
                std::io::stdout().flush().unwrap();
                std::io::stderr().flush().unwrap();
                abort()
            }
            _ => unreachable!(),
        }

        //tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

fn main() {
    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime.block_on(cli_main());
}
