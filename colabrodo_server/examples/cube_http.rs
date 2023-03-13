//! An example NOODLES server that provides cube geometry for clients.

use colabrodo_server::{
    server::*, server_bufferbuilder::*, server_http::*, server_messages::*,
};

/// Build the actual cube geometry.
///
/// This uses the simple helper tools to build a geometry buffer; you don't have to use this feature if you don't want to.
fn make_cube(
    server_state: &mut ServerState,
    asset_server: AssetStorePtr,
) -> GeometryReference {
    let verts = vec![
        VertexMinimal {
            position: [-1.0, -1.0, 1.0],
            normal: [-0.5774, -0.5774, 0.5774],
        },
        VertexMinimal {
            position: [1.0, -1.0, 1.0],
            normal: [0.5774, -0.5774, 0.5774],
        },
        VertexMinimal {
            position: [1.0, 1.0, 1.0],
            normal: [0.5774, 0.5774, 0.5774],
        },
        VertexMinimal {
            position: [-1.0, 1.0, 1.0],
            normal: [-0.5774, 0.5774, 0.5774],
        },
        VertexMinimal {
            position: [-1.0, -1.0, -1.0],
            normal: [-0.5774, -0.5774, -0.5774],
        },
        VertexMinimal {
            position: [1.0, -1.0, -1.0],
            normal: [0.5774, -0.5774, -0.5774],
        },
        VertexMinimal {
            position: [1.0, 1.0, -1.0],
            normal: [0.5774, 0.5774, -0.5774],
        },
        VertexMinimal {
            position: [-1.0, 1.0, -1.0],
            normal: [-0.5774, 0.5774, -0.5774],
        },
    ];

    let index_list = vec![
        // front
        [0, 1, 2],
        [2, 3, 0],
        // right
        [1, 5, 6],
        [6, 2, 1],
        // back
        [7, 6, 5],
        [5, 4, 7],
        // left
        [4, 0, 3],
        [3, 7, 4],
        // bottom
        [4, 5, 1],
        [1, 0, 4],
        // top
        [3, 2, 6],
        [6, 7, 3],
    ];
    let index_list = IndexType::Triangles(index_list.as_slice());

    let test_source = VertexSource {
        name: Some("Cube".to_string()),
        vertex: verts.as_slice(),
        index: index_list,
    };

    // Create a material to go along with this cube
    let material = server_state.materials.new_component(ServerMaterialState {
        name: None,
        mutable: ServerMaterialStateUpdatable {
            pbr_info: Some(ServerPBRInfo {
                base_color: [1.0, 1.0, 0.75, 1.0],
                metallic: Some(1.0),
                roughness: Some(0.1),
                ..Default::default()
            }),
            double_sided: Some(true),
            ..Default::default()
        },
    });

    let pack = test_source.pack_bytes().unwrap();

    // Return a new mesh with this geometry/material
    // Unlike the other cube example, we use a callback to describe how to store the data. Our callback uses the link to the asset server to create a new asset, and publish the URL.

    let url = add_asset(
        asset_server,
        create_asset_id(),
        Asset::new_from_slice(pack.bytes.as_slice()),
    );

    println!("Cube asset URL is at {url}");

    // Return a new mesh with this geometry/material
    test_source
        .build_geometry(
            server_state,
            BufferRepresentation::Bytes(pack.bytes),
            material,
        )
        .unwrap()
}

/// Set up our example state
async fn setup(state: &mut ServerStatePtr, asset_server: AssetStorePtr) {
    let mut state_lock = state.lock().unwrap();

    // Create a cube
    let cube = make_cube(&mut state_lock, asset_server);

    // Create an entity that uses the geometry
    state_lock.entities.new_owned_component(ServerEntityState {
        name: Some("Cube".to_string()),
        mutable: ServerEntityStateUpdatable {
            parent: None,
            transform: None,
            representation: Some(ServerEntityRepresentation::new_render(
                ServerRenderRepresentation {
                    mesh: cube,
                    instances: None,
                },
            )),
            ..Default::default()
        },
    });
}

#[tokio::main]
async fn main() {
    println!("Connect clients to localhost:50000");

    // Set up the web binary asset server
    let asset_server = make_asset_server(AssetServerOptions::default());

    // Proceed as normal
    let opts = ServerOptions::default();

    let mut state = ServerState::new();

    setup(&mut state, asset_server).await;

    server_main(opts, state).await;
}
