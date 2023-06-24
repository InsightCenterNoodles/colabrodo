use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_client::components::*;
use colabrodo_client::delegate::*;
use colabrodo_common::nooid::*;
use colabrodo_common::value_tools::Value;
use colabrodo_server::server::ciborium::value;
use colabrodo_server::server::tokio::runtime;
use colabrodo_server::server::*;

use std::time::Duration;

fn setup_server(state: ServerStatePtr) {
    let mut state_lock = state.lock().unwrap();

    let sig = state_lock.signals.new_owned_component(SignalState {
        name: "test_signal".to_string(),
        doc: Some("This is a test signal".to_string()),
        arg_doc: vec![MethodArg {
            name: "value".to_string(),
            doc: Some("Some value for testing".to_string()),
        }],
        ..Default::default()
    });

    let sig_copy = sig.clone();

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
        state: MethodHandlerSlot::assign(move |m| {
            m.state.lock().unwrap().issue_signal(
                &sig_copy,
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
        state: MethodHandlerSlot::assign(|m| {
            log::info!("Shutdown method invoked");
            m.state.lock().unwrap().shutdown();
            Ok(None)
        }),
    });

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![method, shutdown_m]),
        signals_list: Some(vec![sig]),
        ..Default::default()
    })
}

struct MyClient {}

impl DelegateProvider for MyClient {
    type MethodDelegate = DefaultMethodDelegate;
    type SignalDelegate = DefaultSignalDelegate;
    type BufferDelegate = DefaultBufferDelegate;
    type BufferViewDelegate = DefaultBufferViewDelegate;
    type SamplerDelegate = DefaultSamplerDelegate;
    type ImageDelegate = DefaultImageDelegate;
    type TextureDelegate = DefaultTextureDelegate;
    type MaterialDelegate = DefaultMaterialDelegate;
    type GeometryDelegate = DefaultGeometryDelegate;
    type LightDelegate = DefaultLightDelegate;
    type TableDelegate = DefaultTableDelegate;
    type PlotDelegate = DefaultPlotDelegate;
    type EntityDelegate = DefaultEntityDelegate;
    type DocumentDelegate = MyDocumentDelegate;
}

struct MyDocumentDelegate {
    test_ping_data: Vec<colabrodo_common::value_tools::Value>,
    test_sig_id: SignalID,
    test_ping_id: MethodID,
    test_invoke_id: uuid::Uuid,
}

impl Default for MyDocumentDelegate {
    fn default() -> Self {
        Self {
            test_ping_data: vec![Value::Text("Here is a test".to_string())],
            test_sig_id: Default::default(),
            test_ping_id: Default::default(),
            test_invoke_id: Default::default(),
        }
    }
}

impl DocumentDelegate for MyDocumentDelegate {
    fn on_ready<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
    ) {
        log::info!("Finding signals and methods...");
        self.test_sig_id = client
            .signal_list
            .get_id_by_name("test_signal")
            .expect("Missing required signal");
        self.test_ping_id = client
            .method_list
            .get_id_by_name("ping_pong")
            .expect("Missing required method");

        log::info!("Calling method and waiting for signal");

        self.test_invoke_id = client.invoke_method(
            self.test_ping_id,
            InvokeContext::Document,
            self.test_ping_data.clone(),
        );
    }

    fn on_signal<Provider: DelegateProvider>(
        &mut self,
        id: SignalID,
        _client: &mut ClientState<Provider>,
        args: Vec<ciborium::value::Value>,
    ) {
        assert_eq!(id, self.test_sig_id);

        log::info!("Signal invoked: {args:?}");

        let test = vec![value::Value::Text("Hi there".to_string())];

        assert_eq!(args, test);
    }

    #[allow(unused_variables)]
    fn on_method_reply<Provider: DelegateProvider>(
        &mut self,
        client: &mut ClientState<Provider>,
        invoke_id: uuid::Uuid,
        reply: MessageMethodReply,
    ) {
        log::info!("Got result: {:?}", reply);

        if invoke_id == self.test_invoke_id {
            assert_eq!(
                reply.result.unwrap().as_array().unwrap()[0],
                self.test_ping_data[0]
            );

            log::info!("Issuing shutdown...");

            let shutdown_id =
                client.method_list.get_id_by_name("shutdown").unwrap();

            client.invoke_method(
                self.test_ping_id,
                InvokeContext::Document,
                vec![],
            );

            log::info!("Shutdown sent, waiting for close");
        } else {
            client.shutdown();
        }
    }
}

// =============================================================================

async fn client_path() {
    let channels = start_client_stream(
        "ws://localhost:50000".into(),
        "Simple Client".into(),
    )
    .await
    .unwrap();

    let mut stopper = channels.get_stopper();

    log::info!("Client started...");

    launch_client_worker_thread::<MyClient>(channels);

    stopper.recv().await.unwrap();
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

        setup_server(state.clone());

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

    runtime.block_on(client_path());

    log::info!("Done with client");
}

#[test]
fn main() {
    // for some reason using one runtime causes a stall.
    // in the meantime, test with threads
    env_logger::init();

    let h1 = std::thread::spawn(do_server);

    std::thread::sleep(Duration::from_secs(2));

    let h2 = std::thread::spawn(do_client);

    h2.join().unwrap();
    h1.join().unwrap();
}
