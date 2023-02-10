use colabrodo_core::{
    server_bufferbuilder,
    server_messages::*,
    server_state::{ComponentPtr, ServerState, UserServerState},
};

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

    server_bufferbuilder::create_mesh(server_state, test_source, material)
}

struct CubeServer {
    state: ServerState,

    cube_entity: Option<ComponentPtr<EntityState>>,
}

impl UserServerState for CubeServer {
    fn new(tx: colabrodo_core::server_state::CallbackPtr) -> Self {
        Self {
            state: ServerState::new(tx),
            cube_entity: None,
        }
    }

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
                            mesh: ComponentReference::new(&geom),
                            instances: None,
                        },
                    )),
                    ..Default::default()
                },
            }));
    }

    fn mut_state(&mut self) -> &ServerState {
        &self.state
    }

    fn state(&self) -> &ServerState {
        &self.state
    }

    fn invoke(
        &mut self,
        _method: colabrodo_core::server_state::ComponentPtr<
            colabrodo_core::server_messages::MethodState,
        >,
        _context: colabrodo_core::server_state::InvokeObj,
        _args: Vec<ciborium::value::Value>,
    ) -> colabrodo_core::server_state::MethodResult {
        Err(MethodException::method_not_found(None))
    }
}

fn main() {
    colabrodo_core::server::server_main::<CubeServer>();
}
