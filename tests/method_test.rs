use colabrodo_client::client::*;
use colabrodo_client::components::*;
use colabrodo_common::value_tools::Value;
use colabrodo_server::server::ciborium::value;
use colabrodo_server::server::tokio;
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
        state: MethodHandlerSlot::new_from_closure(move |s, m| {
            s.issue_signal(
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
        state: MethodHandlerSlot::new_from_closure(|s, _| {
            log::info!("Shutdown method invoked");
            s.shutdown();
            Ok(None)
        }),
    });

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![method, shutdown_m]),
        signals_list: Some(vec![sig]),
        ..Default::default()
    })
}

// =============================================================================

async fn wait_for_signal(mut source: SignalRecv) {
    let v = source.recv().await.unwrap();

    log::info!("Signal invoked: {v:?}");

    let test = vec![value::Value::Text("Hi there".to_string())];

    assert_eq!(v, test);
}

async fn client_path() {
    let channels = ClientChannels::new();

    let state = ClientState::new(&channels);

    let handle = tokio::spawn(start_client(
        "ws://localhost:50000".to_string(),
        "Simple Client".to_string(),
        state.clone(),
        channels,
    ));

    log::info!("Client started, waiting for ready...");

    wait_for_start(state.clone()).await;

    log::info!("Ready, subscribing to channel...");

    let signal_channel = {
        let mut lock = state.lock().unwrap();
        let id = lock.signal_list.get_id_by_name("test_signal").unwrap();
        lock.subscribe_signal(id).unwrap()
    };

    log::info!("Finding pingpong method...");

    let ping_pong_id = {
        let lock = state.lock().unwrap();
        lock.method_list.get_id_by_name("ping_pong").unwrap()
    };

    let ping_test = vec![Value::Text("Here is a test".to_string())];

    log::info!("Calling method and waiting for signal");

    let result1 = tokio::join!(
        wait_for_signal(signal_channel),
        invoke_method(
            state.clone(),
            ping_pong_id,
            InvokeContext::Document,
            ping_test.clone()
        )
    );

    log::info!("Got result: {:?}", result1);

    let ping_result = result1.1.unwrap().unwrap();
    let ping_result = ping_result.as_array().unwrap();

    assert_eq!(ping_test[0], ping_result[0]);

    log::info!("Issuing shutdown...");

    let shutdown_id = {
        let lock = state.lock().unwrap();
        lock.method_list.get_id_by_name("shutdown").unwrap()
    };

    // we dont unwrap this as the method could die from server shutdown
    let _ = invoke_method(state, shutdown_id, InvokeContext::Document, vec![])
        .await;

    log::info!("Shutdown sent, waiting for close");

    let r = handle.await;

    if r.is_err() {
        log::error!("Client failed: {r:?}");
        std::process::abort();
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
