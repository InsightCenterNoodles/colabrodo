use colabrodo_client::client::*;
use colabrodo_client::components::*;
use colabrodo_server::server::ciborium::value;
use colabrodo_server::server::tokio;
use colabrodo_server::server::tokio::runtime;
use colabrodo_server::server::*;

use log;
use std::process::abort;
use std::time::Duration;

fn setup(state: ServerStatePtr) {
    let mut state_lock = state.lock().unwrap();

    let sig = state_lock.signals.new_owned_component(SignalState {
        name: "test_signal".to_string(),
        doc: Some("This is a test signal".to_string()),
        arg_doc: vec![MethodArg {
            name: "value".to_string(),
            doc: Some("Some value for testing".to_string()),
        }],
    });

    let method = state_lock.methods.new_owned_component(MethodState {
        name: "ping_pong".to_string(),
        doc: Some(
            "This method just replies with what you send it.".to_string(),
        ),
        return_doc: None,
        arg_doc: vec![MethodArg {
            name: "First arg".to_string(),
            doc: Some("Example doc".to_string()),
        }],
        state: MethodHandlerSlot::new_from_closure(move |s, m| {
            let sig = sig.clone();
            s.issue_signal(
                &sig,
                None,
                vec![value::Value::Text("Hi there".to_string())],
            );
            log::info!("Sending reply...");
            Ok(Some(ciborium::value::Value::Array(m.args)))
        }),
    });

    let shutdown_m = state_lock.methods.new_owned_component(MethodState {
        name: "shutdown".to_string(),
        doc: None,
        return_doc: None,
        arg_doc: vec![],
        state: MethodHandlerSlot::new_from_closure(|s, _| {
            log::info!("Shutdown method invoked");
            s.shutdown();
            Ok(None)
        }),
    });

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![method, shutdown_m]),
        ..Default::default()
    })
}

// ========================================
/*
impl AsyncServer for PingPongServer {
    type CommandType = DefaultCommand;
    type InitType = NoInit;

    fn new(
        &mut state: ServerState,
        tx: colabrodo_server::server_state::CallbackPtr,
        _init: NoInit,
    ) -> Self {
        let sig = state.signals.new_component(SignalState {
            name: "test_signal".to_string(),
            doc: Some("This is a test signal".to_string()),
            arg_doc: vec![MethodArg {
                name: "value".to_string(),
                doc: Some("Some value for testing".to_string()),
            }],
        });

        Self {
            state,
            method_list: Default::default(),
            test_signal: sig,
        }
    }

    fn initialize_state(&mut self) {
        log::debug!("Initializing ping pong state");
        let ptr = self.state.methods.new_component(MethodState {
            name: "ping_pong".to_string(),
            doc: Some(
                "This method just replies with what you send it.".to_string(),
            ),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "First arg".to_string(),
                doc: Some("Example doc".to_string()),
            }],
            ..Default::default()
        });

        self.method_list.insert(ptr.clone(), ping_pong);

        self.state.update_document(
            colabrodo_server::server_messages::ServerDocumentUpdate {
                methods_list: Some(vec![ptr]),
                ..Default::default()
            },
        )
    }

    // If we had some kind of out-of-band messaging to the server, it would be handled here
    fn handle_command(&mut self, _: Self::CommandType) {
        // pass
    }

    fn client_disconnected(&mut self, _id: uuid::Uuid) {
        log::debug!("Last client left, shutting down...");
        self.state.output().send(Output::Shutdown).unwrap();
    }
} */

// =============================================================================

#[derive(Debug)]
struct ExampleState {
    sender: tokio::sync::mpsc::Sender<OutgoingMessage>,

    methods: BasicComponentList<ClientMethodState>,
    signals: BasicComponentList<SignalState>,
    buffers: BasicComponentList<BufferState>,
    buffer_views: BasicComponentList<ClientBufferViewState>,
    samplers: BasicComponentList<SamplerState>,
    images: BasicComponentList<ClientImageState>,
    textures: BasicComponentList<ClientTextureState>,
    materials: BasicComponentList<ClientMaterialState>,
    geometries: BasicComponentList<ClientGeometryState>,
    lights: BasicComponentList<LightState>,
    tables: BasicComponentList<ClientTableState>,
    plots: BasicComponentList<ClientPlotState>,
    entities: BasicComponentList<ClientEntityState>,

    doc: ClientDocumentUpdate,

    counter: i32,
}

impl ExampleState {
    fn decrement(&mut self) {
        self.counter -= 1;

        if self.counter == 1 {
            log::info!("Closing connection to server.");

            let id = self.methods.get_id_by_name("shutdown").unwrap();

            log::info!("Found shutdown message ID: {id:?}");

            self.sender
                .blocking_send(OutgoingMessage::MethodInvoke(
                    ClientInvokeMessage {
                        method: *id,
                        context: None,
                        invoke_id: Some("shutdown_id".to_string()),
                        args: vec![],
                    },
                ))
                .unwrap();
        }
    }
}

struct ExampleStateArgument {}

#[derive(Debug)]
struct ExampleStateCommand {}

impl UserClientState for ExampleState {
    type MethodL = BasicComponentList<ClientMethodState>;
    type SignalL = BasicComponentList<SignalState>;
    type BufferL = BasicComponentList<BufferState>;
    type BufferViewL = BasicComponentList<ClientBufferViewState>;
    type SamplerL = BasicComponentList<SamplerState>;
    type ImageL = BasicComponentList<ClientImageState>;
    type TextureL = BasicComponentList<ClientTextureState>;
    type MaterialL = BasicComponentList<ClientMaterialState>;
    type GeometryL = BasicComponentList<ClientGeometryState>;
    type LightL = BasicComponentList<LightState>;
    type TableL = BasicComponentList<ClientTableState>;
    type PlotL = BasicComponentList<ClientPlotState>;
    type EntityL = BasicComponentList<ClientEntityState>;

    type CommandType = ExampleStateCommand;
    type ArgumentType = ExampleStateArgument;

    fn new(
        _a: Self::ArgumentType,
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        log::info!("Creating client state");
        Self {
            sender: to_server,
            methods: Default::default(),
            signals: Default::default(),
            buffers: Default::default(),
            buffer_views: Default::default(),
            samplers: Default::default(),
            images: Default::default(),
            textures: Default::default(),
            materials: Default::default(),
            geometries: Default::default(),
            lights: Default::default(),
            tables: Default::default(),
            plots: Default::default(),
            entities: Default::default(),
            doc: Default::default(),
            counter: 2,
        }
    }

    fn method_list(&mut self) -> &mut Self::MethodL {
        &mut self.methods
    }

    fn signal_list(&mut self) -> &mut Self::SignalL {
        &mut self.signals
    }

    fn buffer_list(&mut self) -> &mut Self::BufferL {
        &mut self.buffers
    }

    fn buffer_view_list(&mut self) -> &mut Self::BufferViewL {
        &mut self.buffer_views
    }

    fn sampler_list(&mut self) -> &mut Self::SamplerL {
        &mut self.samplers
    }

    fn image_list(&mut self) -> &mut Self::ImageL {
        &mut self.images
    }

    fn texture_list(&mut self) -> &mut Self::TextureL {
        &mut self.textures
    }

    fn material_list(&mut self) -> &mut Self::MaterialL {
        &mut self.materials
    }

    fn geometry_list(&mut self) -> &mut Self::GeometryL {
        &mut self.geometries
    }

    fn light_list(&mut self) -> &mut Self::LightL {
        &mut self.lights
    }

    fn table_list(&mut self) -> &mut Self::TableL {
        &mut self.tables
    }

    fn plot_list(&mut self) -> &mut Self::PlotL {
        &mut self.plots
    }

    fn entity_list(&mut self) -> &mut Self::EntityL {
        &mut self.entities
    }

    fn document_update(&mut self, update: ClientDocumentUpdate) {
        self.doc.update(update);
    }

    fn on_signal_invoke(&mut self, signal: ClientMessageSignalInvoke) {
        log::info!("Signal invoked {signal:?}");

        let _ = match self.signals.find(&signal.id) {
            Some(sig) => sig,
            None => abort(),
        };

        self.decrement()
    }
    fn on_method_reply(&mut self, method_reply: MessageMethodReply) {
        if self.counter == 2 {
            log::info!("Method reply: {method_reply:?}");

            assert_eq!(method_reply.invoke_id, "specific_id");

            assert!(method_reply.method_exception.is_none());

            let reply_data = method_reply.result.unwrap();

            assert_eq!(
                reply_data.as_array().unwrap()[0].as_text().unwrap(),
                "This is specific text"
            );

            self.decrement();
        } else if self.counter < 0 {
            self.sender.blocking_send(OutgoingMessage::Close).unwrap();
        }
    }
    fn on_document_ready(&mut self) {
        log::info!("Document is ready, calling method...");
        let id = self.methods.get_id_by_name("ping_pong").unwrap();

        log::info!("Found message ID: {id:?}");

        let arg = value::Value::Text("This is specific text".to_string());

        self.sender
            .blocking_send(OutgoingMessage::MethodInvoke(ClientInvokeMessage {
                method: *id,
                context: None,
                invoke_id: Some("specific_id".to_string()),
                args: vec![arg],
            }))
            .unwrap();
    }
}

// =============================================================================

fn do_server() {
    log::info!("Starting server");

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let opts = ServerOptions {
            host: "127.0.0.1:50000".to_string(),
        };

        let state = ServerState::new();

        setup(state.clone());

        server_main(opts, state).await;
    });

    log::info!("Done with server");
}

fn do_client() {
    log::info!("Starting client");

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let r = start_client::<ExampleState>(
            "ws://localhost:50000".to_string(),
            "Simple Client".to_string(),
            ExampleStateArgument {},
        )
        .await;

        if r.is_err() {
            log::error!("Client failed: {r:?}");
            std::process::abort();
        }
    });

    log::info!("Done with client");
}

#[test]
fn main() {
    // for some reason using one runtime causes a stall.
    // in the meantime, test with threads
    env_logger::init();

    let h1 = std::thread::spawn(|| do_server());

    std::thread::sleep(Duration::from_secs(2));

    let h2 = std::thread::spawn(|| do_client());

    h2.join().unwrap();
    h1.join().unwrap();
}
