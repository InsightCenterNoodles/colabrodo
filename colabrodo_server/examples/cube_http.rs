//! An example NOODLES server that provides cube geometry for clients.

use colabrodo_server::{
    server::*, server_bufferbuilder::*, server_http::*, server_messages::*,
};

/// Build the actual cube geometry.
///
/// This uses the simple helper tools to build a geometry buffer; you don't have to use this feature if you don't want to.
async fn make_cube(
    server_state: &mut ServerState,
    mut link: AssetServerLink,
) -> ServerGeometryPatch {
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

    let url = link
        .add_asset(
            create_asset_id(),
            Asset::new_from_slice(pack.bytes.as_slice()),
        )
        .await;

    println!("Cube asset URL is at {url}");

    let intermediate = test_source
        .build_states(server_state, BufferRepresentation::Url(url))
        .unwrap();

    // build the cube with our material

    ServerGeometryPatch {
        attributes: intermediate.attributes,
        vertex_count: intermediate.vertex_count,
        indices: intermediate.indices,
        patch_type: intermediate.patch_type,
        material,
    }
}

async fn setup(state: &mut ServerStatePtr, link: AssetServerLink) {
    let mut state_lock = state.lock().unwrap();

    let cube = make_cube(&mut state_lock, link).await;

    let geom = state_lock.geometries.new_component(ServerGeometryState {
        name: Some("Cube Geom".to_string()),
        patches: vec![cube],
    });

    state_lock.entities.new_owned_component(ServerEntityState {
        name: Some("Cube".to_string()),
        mutable: ServerEntityStateUpdatable {
            parent: None,
            transform: None,
            representation: Some(ServerEntityRepresentation::new_render(
                ServerRenderRepresentation {
                    mesh: geom,
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
    let (asset_server, mut link) =
        make_asset_server(AssetServerOptions::default());

    // Launch it
    tokio::spawn(asset_server);

    // Wait for it to start
    link.wait_for_start().await;

    // Proceed as normal
    let opts = ServerOptions::default();

    let mut state = ServerState::new();

    setup(&mut state, link).await;

    server_main(opts, state).await;
}
