mod raycast;

use bevy::{
    prelude::*,
    render::camera::Camera,
    render::color::Color,
    render::mesh::{VertexAttribute, VertexAttributeValues},
    render::pipeline::PrimitiveTopology,
    window::{CursorMoved, WindowId},
};
use raycast::*;
use std::collections::HashMap;

pub struct PickingPlugin;
impl Plugin for PickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PickState>()
            .init_resource::<PickHighlightParams>()
            .add_system(pick_mesh.system())
            .add_system(select_mesh.system())
            .add_system(build_rays.system())
            .add_system(pick_highlighting.system());
    }
}

pub struct DebugPickingPlugin;
impl Plugin for DebugPickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_debug_cursor.system())
            .add_system(update_debug_cursor_position.system());
    }
}

pub struct PickState {
    ray_map: HashMap<PickingGroup, Ray3D>,
    ordered_pick_list_map: HashMap<PickingGroup, Vec<PickIntersection>>,
}

impl PickState {
    pub fn list(&self, group: PickingGroup) -> Option<&Vec<PickIntersection>> {
        self.ordered_pick_list_map.get(&group)
    }
    pub fn top(&self, group: PickingGroup) -> Option<&PickIntersection> {
        match self.ordered_pick_list_map.get(&group) {
            Some(list) => list.first(),
            None => None,
        }
    }
}

impl Default for PickState {
    fn default() -> Self {
        PickState {
            ray_map: HashMap::new(),
            ordered_pick_list_map: HashMap::new(),
        }
    }
}

/// Holds the entity associated with a mesh as well as it's computed intersection from a pick ray cast
#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub struct PickIntersection {
    entity: Entity,
    intersection: Ray3D,
    distance: f32,
}
impl PickIntersection {
    fn new(entity: Entity, intersection: Ray3D, distance: f32) -> Self {
        PickIntersection {
            entity,
            intersection,
            distance,
        }
    }
    /// Entity intersected with
    pub fn entity(&self) -> Entity {
        self.entity
    }
    /// Position vector describing the intersection position.
    pub fn position(&self) -> &Vec3 {
        self.intersection.origin()
    }
    /// Unit vector describing the normal of the intersected triangle.
    pub fn normal(&self) -> &Vec3 {
        self.intersection.direction()
    }
    /// Depth, distance from camera to intersection.
    pub fn distance(&self) -> f32 {
        self.distance
    }
}

#[derive(Debug)]
pub struct PickHighlightParams {
    hover_color: Color,
    selection_color: Color,
}

impl PickHighlightParams {
    pub fn hover_color_mut(&mut self) -> &mut Color {
        &mut self.hover_color
    }
    pub fn selection_color_mut(&mut self) -> &mut Color {
        &mut self.selection_color
    }
    pub fn set_hover_color(&mut self, color: Color) {
        self.hover_color = color;
    }
    pub fn set_selection_color(&mut self, color: Color) {
        self.selection_color = color;
    }
}

impl Default for PickHighlightParams {
    fn default() -> Self {
        PickHighlightParams {
            hover_color: Color::rgb(0.3, 0.5, 0.8),
            selection_color: Color::rgb(0.3, 0.8, 0.5),
        }
    }
}

/// Used to group pickable meshes with a camera into sets
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum PickingGroup {
    None,
    Group(usize),
}

impl Default for PickingGroup {
    fn default() -> Self {
        PickingGroup::Group(0)
    }
}

/// Marks an entity as pickable
#[derive(Debug)]
pub struct PickableMesh {
    group: Vec<PickingGroup>,
    bounding_sphere: Option<BoundingSphere>,
}

impl PickableMesh {
    pub fn new(picking_group: Vec<PickingGroup>) -> Self {
        PickableMesh {
            group: picking_group,
            bounding_sphere: None,
        }
    }
}

impl Default for PickableMesh {
    fn default() -> Self {
        PickableMesh {
            group: [PickingGroup::default()].into(),
            bounding_sphere: None,
        }
    }
}

#[derive(Debug)]
pub enum PickingMethod {
    Cursor(WindowId),
    ScreenSpace(Vec2),
    Center,
}

// Marks an entity to be used for picking, probably a camera
pub struct PickingSource {
    group: PickingGroup,
    pick_method: PickingMethod,
    cursor_events: EventReader<CursorMoved>,
}

impl PickingSource {
    pub fn new(group: PickingGroup, pick_method: PickingMethod) -> Self {
        PickingSource {
            group,
            pick_method,
            ..Default::default()
        }
    }
    pub fn with_group(mut self, group: PickingGroup) -> Self {
        self.group = group;
        self
    }
    pub fn with_pick_method(mut self, pick_method: PickingMethod) -> Self {
        self.pick_method = pick_method;
        self
    }
}

impl Default for PickingSource {
    fn default() -> Self {
        PickingSource {
            group: PickingGroup::Group(0),
            pick_method: PickingMethod::Cursor(WindowId::primary()),
            cursor_events: EventReader::default(),
        }
    }
}

/// Meshes with `SelectableMesh` will have selection state managed
#[derive(Debug)]
pub struct SelectablePickMesh {
    selected: bool,
}

impl SelectablePickMesh {
    pub fn new() -> Self {
        SelectablePickMesh::default()
    }
    pub fn selected(&self) -> bool {
        self.selected
    }
}

impl Default for SelectablePickMesh {
    fn default() -> Self {
        SelectablePickMesh { selected: false }
    }
}

/// Meshes with `HighlightablePickMesh` will be highlighted when hovered over.
/// If the mesh also has the `SelectablePickMesh` component, it will highlight when selected.
#[derive(Debug)]
pub struct HighlightablePickMesh {
    // Stores the initial color of the mesh material prior to selecting/hovering
    initial_color: Option<Color>,
}

impl HighlightablePickMesh {
    pub fn new() -> Self {
        HighlightablePickMesh::default()
    }
}

impl Default for HighlightablePickMesh {
    fn default() -> Self {
        HighlightablePickMesh {
            initial_color: None,
        }
    }
}

struct DebugCursor;

struct DebugCursorMesh;

/// Updates the 3d cursor to be in the pointed world coordinates
fn update_debug_cursor_position(
    pick_state: Res<PickState>,
    mut query: Query<With<DebugCursor, &mut Transform>>,
    mut visibility_query: Query<With<DebugCursorMesh, &mut Draw>>,
) {
    // Set the cursor translation to the top pick's world coordinates
    if let Some(top_pick) = pick_state.top(PickingGroup::default()) {
        let position = top_pick.position();
        let normal = top_pick.normal();
        let up = Vec3::from([0.0, 1.0, 0.0]);
        let axis = up.cross(*normal).normalize();
        let angle = up.dot(*normal).acos();
        let epsilon = 0.0001;
        let new_rotation = if angle.abs() > epsilon {
            Quat::from_axis_angle(axis, angle)
        } else {
            Quat::default()
        };
        let transform_new = Mat4::from_rotation_translation(new_rotation, *position);
        for mut transform in &mut query.iter() {
            *transform.value_mut() = transform_new;
        }
        for mut draw in &mut visibility_query.iter() {
            draw.is_visible = true;
        }
    } else {
        for mut draw in &mut visibility_query.iter() {
            draw.is_visible = false;
        }
    }
}

/// Start up system to create 3d Debug cursor
fn setup_debug_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_matl = materials.add(StandardMaterial {
        albedo: Color::rgb(0.0, 1.0, 0.0),
        shaded: false,
        ..Default::default()
    });
    let cube_size = 0.02;
    let cube_tail_scale = 20.0;
    let ball_size = 0.08;
    commands
        // cursor
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: ball_size,
            })),
            material: debug_matl,
            ..Default::default()
        })
        .with_children(|parent| {
            // child cube
            parent
                .spawn(PbrComponents {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: debug_matl,
                    transform: Transform::from_non_uniform_scale(Vec3::from([
                        1.0,
                        cube_tail_scale,
                        1.0,
                    ]))
                    .with_translation(Vec3::new(
                        0.0,
                        cube_size * cube_tail_scale,
                        0.0,
                    )),
                    ..Default::default()
                })
                .with(DebugCursorMesh);
        })
        .with(DebugCursor)
        .with(DebugCursorMesh);
}

/// Given the current selected and hovered meshes and provided materials, update the meshes with the
/// appropriate materials...
fn pick_highlighting(
    // Resources
    pick_state: Res<PickState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    highlight_params: Res<PickHighlightParams>,
    // Queries
    mut query_picked: Query<(
        &mut HighlightablePickMesh,
        &PickableMesh,
        &Handle<StandardMaterial>,
        Entity,
    )>,
    mut query_selected: Query<(
        &mut HighlightablePickMesh,
        &SelectablePickMesh,
        &Handle<StandardMaterial>,
    )>,
    mut query_selectables: Query<&SelectablePickMesh>,
) {
    // Query selectable entities that have changed
    for (mut highlightable, selectable, material_handle) in &mut query_selected.iter() {
        let current_color = &mut materials.get_mut(material_handle).unwrap().albedo;
        let initial_color = match highlightable.initial_color {
            None => {
                highlightable.initial_color = Some(*current_color);
                *current_color
            }
            Some(color) => color,
        };
        if selectable.selected {
            *current_color = highlight_params.selection_color;
        } else {
            *current_color = initial_color;
        }
    }

    // Query highlightable entities that have changed
    for (mut highlightable, _pickable, material_handle, entity) in &mut query_picked.iter() {
        let current_color = &mut materials.get_mut(material_handle).unwrap().albedo;
        let initial_color = match highlightable.initial_color {
            None => {
                highlightable.initial_color = Some(*current_color);
                *current_color
            }
            Some(color) => color,
        };
        let mut topmost = false;
        if let Some(pick_depth) = pick_state.top(PickingGroup::default()) {
            topmost = pick_depth.entity == entity;
        }
        if topmost {
            *current_color = highlight_params.hover_color;
        } else if let Ok(mut query) = query_selectables.entity(entity) {
            if let Some(selectable) = query.get() {
                if selectable.selected {
                    *current_color = highlight_params.selection_color;
                } else {
                    *current_color = initial_color;
                }
            }
        } else {
            *current_color = initial_color;
        }
    }
}

/// Given the currently hovered mesh, checks for a user click and if detected, sets the selected
/// field in the entity's component to true.
fn select_mesh(
    // Resources
    pick_state: Res<PickState>,
    mouse_button_inputs: Res<Input<MouseButton>>,
    // Queries
    mut query: Query<&mut SelectablePickMesh>,
) {
    if mouse_button_inputs.just_pressed(MouseButton::Left) {
        // Deselect everything
        for mut selectable in &mut query.iter() {
            selectable.selected = false;
        }

        if let Some(pick_depth) = pick_state.top(PickingGroup::default()) {
            if let Ok(mut top_mesh) = query.get_mut::<SelectablePickMesh>(pick_depth.entity) {
                top_mesh.selected = true;
            }
        }
    }
}

fn build_rays(
    // Resources
    mut pick_state: ResMut<PickState>,
    cursor: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    // Queries
    mut pick_source_query: Query<(&mut PickingSource, &Transform, Entity)>,
    camera_query: Query<With<PickingSource, &Camera>>,
) {
    // Collect and calculate pick_ray from all cameras
    pick_state.ray_map.clear();

    for (mut pick_source, transform, entity) in &mut pick_source_query.iter() {
        let group_number = match pick_source.group {
            PickingGroup::Group(num) => num,
            PickingGroup::None => continue,
        };

        match pick_source.pick_method {
            PickingMethod::Cursor(window_id) => {
                let projection_matrix = match camera_query.get::<Camera>(entity) {
                    Ok(camera) => camera.projection_matrix,
                    Err(_) => panic!("The PickingSource in group {} has a {:?} but no associated Camera component", group_number, pick_source.pick_method),
                };
                // Get the cursor position
                let cursor_pos_screen: Vec2 = match pick_source.cursor_events.latest(&cursor) {
                    Some(cursor_moved) => {
                        if cursor_moved.id == window_id {
                            cursor_moved.position
                        } else {
                            continue;
                        }
                    }
                    None => continue,
                };

                // Get current screen size
                let window = windows.get(window_id).unwrap();
                let screen_size = Vec2::from([window.width as f32, window.height as f32]);

                // Normalized device coordinates (NDC) describes cursor position from (-1, -1, -1) to (1, 1, 1)
                let cursor_pos_ndc: Vec3 =
                    ((cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0])).extend(1.0);

                let camera_matrix = *transform.value();
                let (_, _, camera_position) = camera_matrix.to_scale_rotation_translation();

                let ndc_to_world: Mat4 = camera_matrix * projection_matrix.inverse();
                let cursor_position: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc);

                let ray_direction = cursor_position - camera_position;

                let pick_ray = Ray3D::new(camera_position, ray_direction);

                if pick_state
                    .ray_map
                    .insert(pick_source.group, pick_ray)
                    .is_some()
                {
                    panic!(
                        "Multiple PickingSources have been added to pick group: {}",
                        group_number
                    );
                }
            }
            PickingMethod::ScreenSpace(coordinates_ndc) => {
                let projection_matrix = match camera_query.get::<Camera>(entity) {
                    Ok(camera) => camera.projection_matrix,
                    Err(_) => panic!("The PickingSource in group {} has a {:?} but no associated Camera component", group_number, pick_source.pick_method),
                };
                let cursor_pos_ndc: Vec3 = coordinates_ndc.extend(1.0);
                let camera_matrix = *transform.value();
                let (_, _, camera_position) = camera_matrix.to_scale_rotation_translation();

                let ndc_to_world: Mat4 = camera_matrix * projection_matrix.inverse();
                let cursor_position: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc);

                let ray_direction = cursor_position - camera_position;

                let pick_ray = Ray3D::new(camera_position, ray_direction);

                if pick_state
                    .ray_map
                    .insert(pick_source.group, pick_ray)
                    .is_some()
                {
                    panic!(
                        "Multiple PickingSources have been added to pick group: {}",
                        group_number
                    );
                }
            }
            PickingMethod::Center => {
                let pick_position_ndc = Vec3::from([0.0, 0.0, 1.0]);
                let source_transform = *transform.value();
                let pick_position = source_transform.transform_point3(pick_position_ndc);

                let (_, _, source_origin) = source_transform.to_scale_rotation_translation();
                let ray_direction = pick_position - source_origin;

                let pick_ray = Ray3D::new(source_origin, ray_direction);

                if pick_state
                    .ray_map
                    .insert(pick_source.group, pick_ray)
                    .is_some()
                {
                    panic!(
                        "Multiple PickingSources have been added to pick group: {}",
                        group_number
                    );
                }
            }
        }
    }
}

fn pick_mesh(
    // Resources
    mut pick_state: ResMut<PickState>,
    meshes: Res<Assets<Mesh>>,
    // Queries
    mut mesh_query: Query<(&Handle<Mesh>, &Transform, &PickableMesh, Entity, &Draw)>,
) {
    // If there are no rays, then there is nothing to do here
    if pick_state.ray_map.is_empty() {
        return;
    } else {
        // TODO only clear out lists if the corresponding group has a ray
        pick_state.ordered_pick_list_map.clear();
    }

    // Iterate through each pickable mesh in the scene
    for (mesh_handle, transform, pickable, entity, draw) in &mut mesh_query.iter() {
        if !draw.is_visible {
            continue;
        }

        let pick_group = &pickable.group;

        // Check for a pick ray(s) in the group this mesh belongs to
        let mut pick_rays: Vec<(&PickingGroup, Ray3D)> = Vec::new();
        for group in pick_group.iter() {
            if let Some(ray) = pick_state.ray_map.get(group) {
                pick_rays.push((group, *ray));
            }
        }

        if pick_rays.is_empty() {
            continue;
        }

        // Use the mesh handle to get a reference to a mesh asset
        if let Some(mesh) = meshes.get(mesh_handle) {
            if mesh.primitive_topology != PrimitiveTopology::TriangleList {
                continue;
            }

            // Get the vertex positions from the mesh reference resolved from the mesh handle
            let vertex_positions: Vec<[f32; 3]> = mesh
                .attributes
                .iter()
                .filter(|attribute| attribute.name == VertexAttribute::POSITION)
                .filter_map(|attribute| match &attribute.values {
                    VertexAttributeValues::Float3(positions) => Some(positions.clone()),
                    _ => panic!("Unexpected vertex types in VertexAttribute::POSITION"),
                })
                .last()
                .unwrap();

            if let Some(indices) = &mesh.indices {
                // Iterate over the list of pick rays that belong to the same group as this mesh
                for (pick_group, pick_ray) in pick_rays {
                    // The ray cast can hit the same mesh many times, so we need to track which hit is
                    // closest to the camera, and record that.
                    let mut min_pick_distance = f32::MAX;

                    let mesh_to_world = transform.value();
                    let mut pick_intersection: Option<PickIntersection> = None;
                    // Now that we're in the vector of vertex indices, we want to look at the vertex
                    // positions for each triangle, so we'll take indices in chunks of three, where each
                    // chunk of three indices are references to the three vertices of a triangle.
                    for index in indices.chunks(3) {
                        // Make sure this chunk has 3 vertices to avoid a panic.
                        if index.len() != 3 {
                            break;
                        }
                        // Construct a triangle in world space using the mesh data
                        let mut vertices: [Vec3; 3] = [Vec3::zero(), Vec3::zero(), Vec3::zero()];
                        for i in 0..3 {
                            let vertex_pos_local = Vec3::from(vertex_positions[index[i] as usize]);
                            vertices[i] = mesh_to_world.transform_point3(vertex_pos_local)
                        }
                        let triangle = Triangle::from(vertices);
                        // Run the raycast on the ray and triangle
                        if let Some(intersection) = ray_triangle_intersection(
                            &pick_ray,
                            &triangle,
                            RaycastAlgorithm::default(),
                        ) {
                            let distance: f32 =
                                (*intersection.origin() - *pick_ray.origin()).length().abs();
                            if distance < min_pick_distance {
                                min_pick_distance = distance;
                                pick_intersection =
                                    Some(PickIntersection::new(entity, intersection, distance));
                            }
                        }
                    }
                    // Finished going through the current mesh, update pick states
                    if let Some(pick) = pick_intersection {
                        // Make sure the pick list map contains the key
                        match pick_state.ordered_pick_list_map.get_mut(pick_group) {
                            Some(list) => list.push(pick),
                            None => {
                                pick_state
                                    .ordered_pick_list_map
                                    .insert(*pick_group, Vec::from([pick]));
                            }
                        }
                    }
                }
            } else {
                // If we get here the mesh doesn't have an index list!
                panic!(
                    "No index matrix found in mesh {:?}\n{:?}",
                    mesh_handle, mesh
                );
            }
        }
    }
    // Sort the pick list
    for (_group, list) in pick_state.ordered_pick_list_map.iter_mut() {
        list.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}
