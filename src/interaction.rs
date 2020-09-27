use crate::scene::CommandGroup;
use crate::{
    camera::CameraController,
    gui::UiNode,
    scene::{
        ChangeSelectionCommand, EditorScene, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand,
        SceneCommand, Selection,
    },
    GameEngine, Message,
};
use rg3d::gui::message::MessageDirection;
use rg3d::{
    core::{
        color::Color,
        math::{
            aabb::AxisAlignedBoundingBox, mat4::Mat4, plane::Plane, quat::Quat, vec2::Vec2,
            vec3::Vec3,
        },
        pool::Handle,
    },
    gui::message::WidgetMessage,
    renderer::surface::{SurfaceBuilder, SurfaceSharedData},
    scene::{
        base::BaseBuilder, graph::Graph, mesh::MeshBuilder, node::Node, transform::TransformBuilder,
    },
};
use std::sync::{mpsc::Sender, Arc, Mutex};

pub trait InteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    );
    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    );
    fn on_mouse_move(
        &mut self,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        camera: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    );
    fn update(&mut self, editor_scene: &EditorScene, camera: Handle<Node>, engine: &mut GameEngine);
    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine);
}

#[derive(Copy, Clone, Debug)]
pub enum MoveGizmoMode {
    None,
    X,
    Y,
    Z,
    XY,
    YZ,
    ZX,
}

pub struct MoveGizmo {
    mode: MoveGizmoMode,
    origin: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
    xy_plane: Handle<Node>,
    yz_plane: Handle<Node>,
    zx_plane: Handle<Node>,
}

fn make_move_axis(
    graph: &mut Graph,
    rotation: Quat,
    color: Color,
    name_prefix: &str,
) -> (Handle<Node>, Handle<Node>) {
    let axis = graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name_prefix.to_owned() + "Axis")
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(rotation)
                        .build(),
                ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceSharedData::make_cylinder(10, 0.015, 1.0, true, Default::default()),
        )))
        .with_color(color)
        .build()])
        .build(),
    ));
    let arrow = graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name_prefix.to_owned() + "Arrow")
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vec3::new(0.0, 1.0, 0.0))
                        .build(),
                ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceSharedData::make_cone(10, 0.05, 0.1, Default::default()),
        )))
        .with_color(color)
        .build()])
        .build(),
    ));
    graph.link_nodes(arrow, axis);
    (axis, arrow)
}

fn create_quad_plane(graph: &mut Graph, transform: Mat4, color: Color, name: &str) -> Handle<Node> {
    graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name)
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_scale(Vec3::new(0.15, 0.15, 0.15))
                        .build(),
                ),
        )
        .with_surfaces(vec![{
            SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_quad(
                transform,
            ))))
            .with_color(color)
            .build()
        }])
        .build(),
    ))
}

impl MoveGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Base(
            BaseBuilder::new()
                .with_name("Origin")
                .with_visibility(false)
                .build(),
        ));

        graph.link_nodes(origin, editor_scene.root);

        let (x_axis, x_arrow) = make_move_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_move_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_move_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        let xy_transform = Mat4::translate(Vec3::new(-0.5, 0.5, 0.0))
            * Mat4::from_quat(Quat::from_axis_angle(
                Vec3::new(1.0, 0.0, 0.0),
                90.0f32.to_radians(),
            ));
        let xy_plane = create_quad_plane(graph, xy_transform, Color::BLUE, "XYPlane");
        graph.link_nodes(xy_plane, origin);

        let yz_transform = Mat4::translate(Vec3::new(0.0, 0.5, 0.5))
            * Mat4::from_quat(Quat::from_axis_angle(
                Vec3::new(0.0, 0.0, 1.0),
                90.0f32.to_radians(),
            ));
        let yz_plane = create_quad_plane(graph, yz_transform, Color::RED, "YZPlane");
        graph.link_nodes(yz_plane, origin);

        let zx_plane = create_quad_plane(
            graph,
            Mat4::translate(Vec3::new(-0.5, 0.0, 0.5)),
            Color::GREEN,
            "ZXPlane",
        );
        graph.link_nodes(zx_plane, origin);

        Self {
            mode: MoveGizmoMode::None,
            origin,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
            zx_plane,
            yz_plane,
            xy_plane,
        }
    }

    pub fn set_mode(&mut self, mode: MoveGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.x_arrow].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.y_arrow].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);
        graph[self.z_arrow].as_mesh_mut().set_color(Color::BLUE);
        graph[self.zx_plane].as_mesh_mut().set_color(Color::GREEN);
        graph[self.yz_plane].as_mesh_mut().set_color(Color::RED);
        graph[self.xy_plane].as_mesh_mut().set_color(Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            MoveGizmoMode::X => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
                graph[self.x_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::Y => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
                graph[self.y_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::Z => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
                graph[self.z_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::XY => {
                graph[self.xy_plane].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::YZ => {
                graph[self.yz_plane].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::ZX => {
                graph[self.zx_plane].as_mesh_mut().set_color(yellow);
            }
            _ => (),
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis || picked == self.x_arrow {
            self.set_mode(MoveGizmoMode::X, graph);
            true
        } else if picked == self.y_axis || picked == self.y_arrow {
            self.set_mode(MoveGizmoMode::Y, graph);
            true
        } else if picked == self.z_axis || picked == self.z_arrow {
            self.set_mode(MoveGizmoMode::Z, graph);
            true
        } else if picked == self.zx_plane {
            self.set_mode(MoveGizmoMode::ZX, graph);
            true
        } else if picked == self.xy_plane {
            self.set_mode(MoveGizmoMode::XY, graph);
            true
        } else if picked == self.yz_plane {
            self.set_mode(MoveGizmoMode::YZ, graph);
            true
        } else {
            self.set_mode(MoveGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_offset(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        engine: &GameEngine,
    ) -> Vec3 {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);
        let node_global_transform = graph[self.origin].global_transform();
        let node_local_transform = graph[self.origin].local_transform().matrix();

        if let Node::Camera(camera) = &graph[camera] {
            let dlook = node_global_transform.position() - camera.global_position();
            let inv_node_transform = node_global_transform.inverse().unwrap_or_default();

            // Create two rays in object space.
            let initial_ray = camera
                .make_ray(mouse_position, screen_size)
                .transform(inv_node_transform);
            let offset_ray = camera
                .make_ray(mouse_position + mouse_offset, screen_size)
                .transform(inv_node_transform);

            // Select plane by current active mode.
            let plane = match self.mode {
                MoveGizmoMode::None => return Vec3::ZERO,
                MoveGizmoMode::X => {
                    Plane::from_normal_and_point(&Vec3::new(0.0, dlook.y, dlook.z), &Vec3::ZERO)
                }
                MoveGizmoMode::Y => {
                    Plane::from_normal_and_point(&Vec3::new(dlook.x, 0.0, dlook.z), &Vec3::ZERO)
                }
                MoveGizmoMode::Z => {
                    Plane::from_normal_and_point(&Vec3::new(dlook.x, dlook.y, 0.0), &Vec3::ZERO)
                }
                MoveGizmoMode::YZ => Plane::from_normal_and_point(&Vec3::RIGHT, &Vec3::ZERO),
                MoveGizmoMode::ZX => Plane::from_normal_and_point(&Vec3::UP, &Vec3::ZERO),
                MoveGizmoMode::XY => Plane::from_normal_and_point(&Vec3::LOOK, &Vec3::ZERO),
            }
            .unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate offset.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    let offset = match self.mode {
                        MoveGizmoMode::None => unreachable!(),
                        MoveGizmoMode::X => Vec3::new(delta.x, 0.0, 0.0),
                        MoveGizmoMode::Y => Vec3::new(0.0, delta.y, 0.0),
                        MoveGizmoMode::Z => Vec3::new(0.0, 0.0, delta.z),
                        MoveGizmoMode::XY => Vec3::new(delta.x, delta.y, 0.0),
                        MoveGizmoMode::YZ => Vec3::new(0.0, delta.y, delta.z),
                        MoveGizmoMode::ZX => Vec3::new(delta.x, 0.0, delta.z),
                    };
                    // Make sure offset will be in local coordinates.
                    return node_local_transform.transform_vector_normal(offset);
                }
            }
        }

        Vec3::ZERO
    }

    pub fn sync_transform(&self, graph: &mut Graph, selection: &Selection, scale: Vec3) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            graph[self.origin]
                .set_visibility(true)
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

fn distance_scale_factor(fov: f32) -> f32 {
    fov.tan() * 0.1
}

pub struct MoveInteractionMode {
    initial_positions: Vec<Vec3>,
    move_gizmo: MoveGizmo,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl MoveInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            initial_positions: Default::default(),
            move_gizmo: MoveGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionMode for MoveInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node =
            camera_controller.pick(mouse_pos, editor_scene, engine, true, |handle, _| {
                handle != camera && handle != camera_pivot
            });

        if self
            .move_gizmo
            .handle_pick(editor_node, editor_scene, engine)
        {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_positions = editor_scene.selection.local_positions(graph);
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        if self.interacting {
            if !editor_scene.selection.is_empty() {
                self.interacting = false;
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let current_positions = editor_scene.selection.local_positions(graph);
                if current_positions != self.initial_positions {
                    let commands = CommandGroup::from(
                        editor_scene
                            .selection
                            .nodes()
                            .iter()
                            .zip(current_positions.iter().zip(self.initial_positions.iter()))
                            .map(|(&node, (&new_pos, &old_pos))| {
                                SceneCommand::MoveNode(MoveNodeCommand::new(node, old_pos, new_pos))
                            })
                            .collect::<Vec<SceneCommand>>(),
                    );
                    // Commit changes.
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                            commands,
                        )))
                        .unwrap();
                }
            }
        } else {
            let picked =
                camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            let new_selection =
                if engine.user_interface.keyboard_modifiers().control && picked.is_some() {
                    let mut selection = editor_scene.selection.clone();
                    selection.insert_or_exclude(picked);
                    selection
                } else {
                    Selection::single_or_empty(picked)
                };
            if new_selection != editor_scene.selection {
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                        ChangeSelectionCommand::new(new_selection, editor_scene.selection.clone()),
                    )))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        camera: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        if self.interacting {
            let node_offset = self.move_gizmo.calculate_offset(
                editor_scene,
                camera,
                mouse_offset,
                mouse_position,
                engine,
            );
            editor_scene
                .selection
                .offset(&mut engine.scenes[editor_scene.scene].graph, node_offset);
        }
    }

    fn update(
        &mut self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        if !editor_scene.selection.is_empty() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let distance = distance_scale_factor(graph[camera].as_camera().fov())
                * graph[self.move_gizmo.origin]
                    .global_position()
                    .distance(&graph[camera].global_position());
            let scale = Vec3::new(distance, distance, distance);
            self.move_gizmo
                .sync_transform(graph, &editor_scene.selection, scale);
            self.move_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.move_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.move_gizmo.set_visible(graph, false);
    }
}

pub enum ScaleGizmoMode {
    None,
    X,
    Y,
    Z,
    Uniform,
}

pub struct ScaleGizmo {
    mode: ScaleGizmoMode,
    origin: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_scale_axis(
    graph: &mut Graph,
    rotation: Quat,
    color: Color,
    name_prefix: &str,
) -> (Handle<Node>, Handle<Node>) {
    let axis = graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name_prefix.to_owned() + "Axis")
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(rotation)
                        .build(),
                ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceSharedData::make_cylinder(10, 0.015, 1.0, true, Default::default()),
        )))
        .with_color(color)
        .build()])
        .build(),
    ));
    let arrow = graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name_prefix.to_owned() + "Arrow")
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vec3::new(0.0, 1.0, 0.0))
                        .build(),
                ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceSharedData::make_cube(Mat4::scale(Vec3::new(0.1, 0.1, 0.1))),
        )))
        .with_color(color)
        .build()])
        .build(),
    ));
    graph.link_nodes(arrow, axis);
    (axis, arrow)
}

impl ScaleGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Mesh(
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Origin")
                    .with_visibility(false),
            )
            .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
                SurfaceSharedData::make_cube(Mat4::scale(Vec3::new(0.1, 0.1, 0.1))),
            )))
            .build()])
            .build(),
        ));

        graph.link_nodes(origin, editor_scene.root);

        let (x_axis, x_arrow) = make_scale_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_scale_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_scale_axis(
            graph,
            Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        Self {
            mode: ScaleGizmoMode::None,
            origin,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn set_mode(&mut self, mode: ScaleGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        graph[self.origin].as_mesh_mut().set_color(Color::WHITE);
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.x_arrow].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.y_arrow].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);
        graph[self.z_arrow].as_mesh_mut().set_color(Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            ScaleGizmoMode::None => (),
            ScaleGizmoMode::X => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
                graph[self.x_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Y => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
                graph[self.y_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Z => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
                graph[self.z_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Uniform => {
                graph[self.origin].as_mesh_mut().set_color(yellow);
            }
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis || picked == self.x_arrow {
            self.set_mode(ScaleGizmoMode::X, graph);
            true
        } else if picked == self.y_axis || picked == self.y_arrow {
            self.set_mode(ScaleGizmoMode::Y, graph);
            true
        } else if picked == self.z_axis || picked == self.z_arrow {
            self.set_mode(ScaleGizmoMode::Z, graph);
            true
        } else if picked == self.origin {
            self.set_mode(ScaleGizmoMode::Uniform, graph);
            true
        } else {
            self.set_mode(ScaleGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_scale_delta(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        engine: &GameEngine,
    ) -> Vec3 {
        let graph = &engine.scenes[editor_scene.scene].graph;
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);
        let node_global_transform = graph[self.origin].global_transform();

        if let Node::Camera(camera) = &graph[camera] {
            let dlook = node_global_transform.position() - camera.global_position();
            let inv_node_transform = node_global_transform.inverse().unwrap_or_default();

            // Create two rays in object space.
            let initial_ray = camera
                .make_ray(mouse_position, screen_size)
                .transform(inv_node_transform);
            let offset_ray = camera
                .make_ray(mouse_position + mouse_offset, screen_size)
                .transform(inv_node_transform);

            // Select plane by current active mode.
            let plane = match self.mode {
                ScaleGizmoMode::None => return Vec3::ZERO,
                ScaleGizmoMode::X => {
                    Plane::from_normal_and_point(&Vec3::new(0.0, dlook.y, dlook.z), &Vec3::ZERO)
                }
                ScaleGizmoMode::Y => {
                    Plane::from_normal_and_point(&Vec3::new(dlook.x, 0.0, dlook.z), &Vec3::ZERO)
                }
                ScaleGizmoMode::Z => {
                    Plane::from_normal_and_point(&Vec3::new(dlook.x, dlook.y, 0.0), &Vec3::ZERO)
                }
                ScaleGizmoMode::Uniform => Plane::from_normal_and_point(&dlook, &Vec3::ZERO),
            }
            .unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate scale delta.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    return match self.mode {
                        ScaleGizmoMode::None => unreachable!(),
                        ScaleGizmoMode::X => Vec3::new(-delta.x, 0.0, 0.0),
                        ScaleGizmoMode::Y => Vec3::new(0.0, delta.y, 0.0),
                        ScaleGizmoMode::Z => Vec3::new(0.0, 0.0, delta.z),
                        ScaleGizmoMode::Uniform => {
                            // TODO: Still may behave weird.
                            let amount = delta.len() * (delta.y + delta.x + delta.z).signum();
                            Vec3::new(amount, amount, amount)
                        }
                    };
                }
            }
        }

        Vec3::ZERO
    }

    pub fn sync_transform(&self, graph: &mut Graph, selection: &Selection, scale: Vec3) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            graph[self.origin]
                .set_visibility(true)
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

pub struct ScaleInteractionMode {
    initial_scales: Vec<Vec3>,
    scale_gizmo: ScaleGizmo,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl ScaleInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            initial_scales: Default::default(),
            scale_gizmo: ScaleGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionMode for ScaleInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node =
            camera_controller.pick(mouse_pos, editor_scene, engine, true, |handle, _| {
                handle != camera && handle != camera_pivot
            });

        if self
            .scale_gizmo
            .handle_pick(editor_node, editor_scene, engine)
        {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_scales = editor_scene.selection.local_scales(graph);
        } else {
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        if self.interacting {
            if !editor_scene.selection.is_empty() {
                self.interacting = false;
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let current_scales = editor_scene.selection.local_scales(graph);
                if current_scales != self.initial_scales {
                    // Commit changes.
                    let commands = CommandGroup::from(
                        editor_scene
                            .selection
                            .nodes()
                            .iter()
                            .zip(self.initial_scales.iter().zip(current_scales.iter()))
                            .map(|(&node, (&old_scale, &new_scale))| {
                                SceneCommand::ScaleNode(ScaleNodeCommand::new(
                                    node, old_scale, new_scale,
                                ))
                            })
                            .collect::<Vec<SceneCommand>>(),
                    );
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                            commands,
                        )))
                        .unwrap();
                }
            }
        } else {
            let picked =
                camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            let new_selection =
                if engine.user_interface.keyboard_modifiers().control && picked.is_some() {
                    let mut selection = editor_scene.selection.clone();
                    selection.insert_or_exclude(picked);
                    selection
                } else {
                    Selection::single_or_empty(picked)
                };
            if new_selection != editor_scene.selection {
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                        ChangeSelectionCommand::new(new_selection, editor_scene.selection.clone()),
                    )))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        camera: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        if self.interacting {
            let scale_delta = self.scale_gizmo.calculate_scale_delta(
                editor_scene,
                camera,
                mouse_offset,
                mouse_position,
                engine,
            );
            for &node in editor_scene.selection.nodes().iter() {
                let transform = engine.scenes[editor_scene.scene].graph[node].local_transform_mut();
                let initial_scale = transform.scale();
                let sx = (initial_scale.x * (1.0 + scale_delta.x)).max(std::f32::EPSILON);
                let sy = (initial_scale.y * (1.0 + scale_delta.y)).max(std::f32::EPSILON);
                let sz = (initial_scale.z * (1.0 + scale_delta.z)).max(std::f32::EPSILON);
                transform.set_scale(Vec3::new(sx, sy, sz));
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        if !editor_scene.selection.is_empty() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let distance = distance_scale_factor(graph[camera].as_camera().fov())
                * graph[self.scale_gizmo.origin]
                    .global_position()
                    .distance(&graph[camera].global_position());
            let scale = Vec3::new(distance, distance, distance);
            self.scale_gizmo
                .sync_transform(graph, &editor_scene.selection, scale);
            self.scale_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.scale_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.scale_gizmo.set_visible(graph, false);
    }
}

pub enum RotateGizmoMode {
    Pitch,
    Yaw,
    Roll,
}

pub struct RotationGizmo {
    mode: RotateGizmoMode,
    origin: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_rotation_ribbon(
    graph: &mut Graph,
    rotation: Quat,
    color: Color,
    name: &str,
) -> Handle<Node> {
    graph.add_node(Node::Mesh(
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name(name)
                .with_depth_offset(0.5)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(rotation)
                        .build(),
                ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceSharedData::make_cylinder(
                30,
                0.5,
                0.05,
                false,
                Mat4::translate(Vec3::new(0.0, -0.05, 0.0)),
            ),
        )))
        .with_color(color)
        .build()])
        .build(),
    ))
}

impl RotationGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Mesh(
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Origin")
                    .with_depth_offset(0.5)
                    .with_visibility(false),
            )
            .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
                SurfaceSharedData::make_sphere(10, 10, 0.1),
            )))
            .with_color(Color::opaque(100, 100, 100))
            .build()])
            .build(),
        ));

        graph.link_nodes(origin, editor_scene.root);

        let x_axis = make_rotation_ribbon(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let y_axis = make_rotation_ribbon(
            graph,
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let z_axis = make_rotation_ribbon(
            graph,
            Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        Self {
            mode: RotateGizmoMode::Pitch,
            origin,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn set_mode(&mut self, mode: RotateGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        graph[self.origin].as_mesh_mut().set_color(Color::WHITE);
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            RotateGizmoMode::Pitch => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Yaw => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Roll => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
            }
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis {
            self.set_mode(RotateGizmoMode::Pitch, graph);
            true
        } else if picked == self.y_axis {
            self.set_mode(RotateGizmoMode::Yaw, graph);
            true
        } else if picked == self.z_axis {
            self.set_mode(RotateGizmoMode::Roll, graph);
            true
        } else {
            false
        }
    }

    pub fn calculate_rotation_delta(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        engine: &GameEngine,
    ) -> Quat {
        let graph = &engine.scenes[editor_scene.scene].graph;
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);

        if let Node::Camera(camera) = &graph[camera] {
            let transform = graph[self.origin].global_transform();

            let initial_ray = camera.make_ray(mouse_position, screen_size);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, screen_size);

            let oriented_axis = match self.mode {
                RotateGizmoMode::Pitch => transform.side(),
                RotateGizmoMode::Yaw => transform.up(),
                RotateGizmoMode::Roll => transform.look(),
            };

            let plane = Plane::from_normal_and_point(&oriented_axis, &transform.position())
                .unwrap_or_default();

            if let Some(old_pos) = initial_ray.plane_intersection_point(&plane) {
                if let Some(new_pos) = offset_ray.plane_intersection_point(&plane) {
                    let center = transform.position();
                    let old = (old_pos - center).normalized().unwrap_or_default();
                    let new = (new_pos - center).normalized().unwrap_or_default();

                    let angle_delta = old.dot(&new).max(-1.0).min(1.0).acos();
                    let sign = old.cross(&new).dot(&oriented_axis).signum();

                    let static_axis = match self.mode {
                        RotateGizmoMode::Pitch => Vec3::RIGHT,
                        RotateGizmoMode::Yaw => Vec3::UP,
                        RotateGizmoMode::Roll => Vec3::LOOK,
                    };
                    return Quat::from_axis_angle(static_axis, sign * angle_delta);
                }
            }
        }

        Quat::default()
    }

    pub fn sync_transform(&self, graph: &mut Graph, selection: &Selection, scale: Vec3) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            graph[self.origin]
                .set_visibility(true)
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

pub struct RotateInteractionMode {
    initial_rotations: Vec<Quat>,
    rotation_gizmo: RotationGizmo,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl RotateInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            initial_rotations: Default::default(),
            rotation_gizmo: RotationGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionMode for RotateInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node =
            camera_controller.pick(mouse_pos, editor_scene, engine, true, |handle, _| {
                handle != camera && handle != camera_pivot
            });

        if self
            .rotation_gizmo
            .handle_pick(editor_node, editor_scene, engine)
        {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_rotations = editor_scene.selection.local_rotations(graph);
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        if self.interacting {
            if !editor_scene.selection.is_empty() {
                self.interacting = false;
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let current_rotation = editor_scene.selection.local_rotations(graph);
                if current_rotation != self.initial_rotations {
                    let commands = CommandGroup::from(
                        editor_scene
                            .selection
                            .nodes()
                            .iter()
                            .zip(self.initial_rotations.iter().zip(current_rotation.iter()))
                            .map(|(&node, (&old_rotation, &new_rotation))| {
                                SceneCommand::RotateNode(RotateNodeCommand::new(
                                    node,
                                    old_rotation,
                                    new_rotation,
                                ))
                            })
                            .collect::<Vec<SceneCommand>>(),
                    );
                    // Commit changes.
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                            commands,
                        )))
                        .unwrap();
                }
            }
        } else {
            let picked =
                camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            let new_selection =
                if engine.user_interface.keyboard_modifiers().control && picked.is_some() {
                    let mut selection = editor_scene.selection.clone();
                    selection.insert_or_exclude(picked);
                    selection
                } else {
                    Selection::single_or_empty(picked)
                };
            if new_selection != editor_scene.selection {
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                        ChangeSelectionCommand::new(new_selection, editor_scene.selection.clone()),
                    )))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vec2,
        mouse_position: Vec2,
        camera: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        if self.interacting {
            let rotation_delta = self.rotation_gizmo.calculate_rotation_delta(
                editor_scene,
                camera,
                mouse_offset,
                mouse_position,
                engine,
            );
            for &node in editor_scene.selection.nodes().iter() {
                let transform = engine.scenes[editor_scene.scene].graph[node].local_transform_mut();
                let rotation = transform.rotation();
                transform.set_rotation(rotation * rotation_delta);
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        if !editor_scene.selection.is_empty() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let distance = distance_scale_factor(graph[camera].as_camera().fov())
                * graph[self.rotation_gizmo.origin]
                    .global_position()
                    .distance(&graph[camera].global_position());
            let scale = Vec3::new(distance, distance, distance);
            self.rotation_gizmo
                .sync_transform(graph, &editor_scene.selection, scale);
            self.rotation_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.rotation_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.rotation_gizmo.set_visible(graph, false);
    }
}

pub struct SelectInteractionMode {
    preview: Handle<UiNode>,
    selection_frame: Handle<UiNode>,
    message_sender: Sender<Message>,
    stack: Vec<Handle<Node>>,
    click_pos: Vec2,
}

impl SelectInteractionMode {
    pub fn new(
        preview: Handle<UiNode>,
        selection_frame: Handle<UiNode>,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            preview,
            selection_frame,
            message_sender,
            stack: Vec::new(),
            click_pos: Vec2::ZERO,
        }
    }
}

impl InteractionMode for SelectInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_scene: &EditorScene,
        _camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        mouse_pos: Vec2,
    ) {
        self.click_pos = mouse_pos;
        let ui = &mut engine.user_interface;
        ui.send_message(WidgetMessage::visibility(
            self.selection_frame,
            MessageDirection::ToWidget,
            true,
        ));
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            mouse_pos,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &EditorScene,
        camera_controller: &mut CameraController,
        engine: &mut GameEngine,
        _mouse_pos: Vec2,
    ) {
        let scene = &engine.scenes[editor_scene.scene];
        let camera = scene.graph[camera_controller.camera].as_camera();
        let preview_screen_bounds = engine.user_interface.node(self.preview).screen_bounds();
        let frame_screen_bounds = engine
            .user_interface
            .node(self.selection_frame)
            .screen_bounds();
        let relative_bounds =
            frame_screen_bounds.translate(-preview_screen_bounds.x, -preview_screen_bounds.y);
        self.stack.clear();
        self.stack.push(scene.graph.get_root());
        let mut selection = Selection::default();
        while let Some(handle) = self.stack.pop() {
            let node = &scene.graph[handle];
            if handle == editor_scene.root {
                continue;
            }
            if handle == scene.graph.get_root() {
                self.stack.extend_from_slice(node.children());
                continue;
            }
            let aabb = match node {
                Node::Base(_) => AxisAlignedBoundingBox::UNIT,
                Node::Light(_) => AxisAlignedBoundingBox::UNIT,
                Node::Camera(_) => AxisAlignedBoundingBox::UNIT,
                Node::Mesh(mesh) => mesh.bounding_box(),
                Node::Sprite(_) => AxisAlignedBoundingBox::UNIT,
                Node::ParticleSystem(_) => AxisAlignedBoundingBox::UNIT,
            };

            let fsize = Vec2::new(
                engine.renderer.get_frame_size().0 as f32,
                engine.renderer.get_frame_size().1 as f32,
            );
            for screen_corner in aabb
                .corners()
                .iter()
                .filter_map(|&p| camera.project(p + node.global_position(), fsize))
            {
                if relative_bounds.contains(screen_corner.x, screen_corner.y) {
                    selection.insert_or_exclude(handle);
                    break;
                }
            }

            self.stack.extend_from_slice(node.children());
        }
        if !selection.is_empty() && selection != editor_scene.selection {
            self.message_sender
                .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                    ChangeSelectionCommand::new(selection, editor_scene.selection.clone()),
                )))
                .unwrap();
        }
        engine
            .user_interface
            .send_message(WidgetMessage::visibility(
                self.selection_frame,
                MessageDirection::ToWidget,
                false,
            ));
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vec2,
        mouse_position: Vec2,
        _camera: Handle<Node>,
        _editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        let ui = &mut engine.user_interface;
        let width = mouse_position.x - self.click_pos.x;
        let height = mouse_position.y - self.click_pos.y;

        let position = Vec2 {
            x: if width < 0.0 {
                mouse_position.x
            } else {
                self.click_pos.x
            },
            y: if height < 0.0 {
                mouse_position.y
            } else {
                self.click_pos.y
            },
        };
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            position,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            width.abs(),
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            height.abs(),
        ));
    }

    fn update(
        &mut self,
        _editor_scene: &EditorScene,
        _camera: Handle<Node>,
        _engine: &mut GameEngine,
    ) {
    }

    fn deactivate(&mut self, _editor_scene: &EditorScene, _engine: &mut GameEngine) {}
}

/// Helper enum to be able to access interaction modes in array directly.
#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
#[repr(usize)]
pub enum InteractionModeKind {
    Select = 0,
    Move = 1,
    Scale = 2,
    Rotate = 3,
}
