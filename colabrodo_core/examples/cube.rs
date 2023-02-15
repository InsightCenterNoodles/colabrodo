//! An example NOODLES server that provides cube geometry for clients.

use colabrodo_core::{
    server::{AsyncServer, DefaultCommand, ServerOptions},
    server_bufferbuilder,
    server_messages::*,
    server_state::{ServerState, UserServerState},
};

/// Build the actual cube geometry.
///
/// This uses the simple helper tools to build a geometry buffer; you don't have to use this feature if you don't want to.
fn make_cube(server_state: &mut ServerState) -> GeometryPatch {
    let mut test_source = server_bufferbuilder::VertexSource::default();

    test_source.name = "Cube".to_string();

    test_source.positions = vec![
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
        //
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
    ];

    test_source.normals = vec![
        [-0.5774, -0.5774, 0.5774],
        [0.5774, -0.5774, 0.5774],
        [0.5774, 0.5774, 0.5774],
        [-0.5774, 0.5774, 0.5774],
        //
        [-0.5774, -0.5774, -0.5774],
        [0.5774, -0.5774, -0.5774],
        [0.5774, 0.5774, -0.5774],
        [-0.5774, 0.5774, -0.5774],
    ];

    test_source.triangles = vec![
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

    // Create a material to go along with this cube
    let material = server_state.materials.new_component(MaterialState {
        name: None,
        extra: MaterialStateUpdatable {
            pbr_info: Some(PBRInfo {
                base_color: [1.0, 1.0, 0.75, 1.0],
                metallic: Some(1.0),
                roughness: Some(0.1),
                ..Default::default()
            }),
            double_sided: Some(true),
            ..Default::default()
        },
    });

    // Return a new mesh with this geometry/material
    let intermediate =
        server_bufferbuilder::create_mesh(server_state, test_source);

    // build the cube with our material

    GeometryPatch {
        attributes: intermediate.attributes,
        vertex_count: intermediate.vertex_count,
        indices: intermediate.indices,
        patch_type: intermediate.patch_type,
        material: material,
    }
}

/// Example implementation of a server
struct CubeServer {
    state: ServerState,

    cube_entity: Option<ComponentReference<EntityState>>,
}

/// All server states should use this trait...
impl UserServerState for CubeServer {
    /// Some code will need mutable access to the core server state
    fn mut_state(&mut self) -> &ServerState {
        &self.state
    }

    /// Some code will need non-mutable access to the core server state
    fn state(&self) -> &ServerState {
        &self.state
    }

    /// When a method invoke is received, it will be validated and then passed here for processing.
    fn invoke(
        &mut self,
        _method: ComponentReference<MethodState>,
        _context: colabrodo_core::server_state::InvokeObj,
        _args: Vec<ciborium::value::Value>,
    ) -> colabrodo_core::server_state::MethodResult {
        Err(MethodException::method_not_found(None))
    }
}

/// And servers that use the provided tokio infrastructure should impl this trait, too...
impl AsyncServer for CubeServer {
    type CommandType = DefaultCommand;

    /// When needed the network server will create our struct with this function
    fn new(tx: colabrodo_core::server_state::CallbackPtr) -> Self {
        Self {
            state: ServerState::new(tx),
            cube_entity: None,
        }
    }

    /// Any additional state can be created here.
    fn initialize_state(&mut self) {
        let cube = make_cube(&mut self.state);

        let geom = self.state.geometries.new_component(GeometryState {
            name: Some("Cube Geom".to_string()),
            patches: vec![cube],
        });

        self.cube_entity =
            Some(self.state.entities.new_component(EntityState {
                name: Some("Cube".to_string()),
                extra: EntityStateUpdatable {
                    parent: None,
                    transform: None,
                    representation: Some(EntityRepresentation::Render(
                        RenderRepresentation {
                            mesh: geom,
                            instances: None,
                        },
                    )),
                    ..Default::default()
                },
            }));
    }

    // If we had some kind of out-of-band messaging to the server, it would be handled here
    fn handle_command(&mut self, _: Self::CommandType) {
        // pass
    }
}

#[tokio::main]
async fn main() {
    println!("Connect clients to localhost:50000");
    let opts = ServerOptions::default();
    colabrodo_core::server::server_main::<CubeServer>(opts).await;
}
