use bevy::prelude::*;

use crate::{input::PointerMultiselect, output, PointerId};

/// Tracks the current selection state of the entity.
#[derive(Component, Debug, Default, Clone)]
pub struct PickSelection {
    pub is_selected: bool,
}

#[derive(Component, Debug, Copy, Clone)]
pub enum PointerSelectionEvent {
    JustSelected(Entity),
    JustDeselected(Entity),
}
impl PointerSelectionEvent {
    pub fn receive(
        mut events: EventReader<PointerSelectionEvent>,
        mut selectables: Query<&mut PickSelection>,
    ) {
        for event in events.iter() {
            match event {
                PointerSelectionEvent::JustSelected(entity) => {
                    if let Ok(mut s) = selectables.get_mut(*entity) {
                        s.is_selected = true
                    }
                }
                PointerSelectionEvent::JustDeselected(entity) => {
                    if let Ok(mut s) = selectables.get_mut(*entity) {
                        s.is_selected = false
                    }
                }
            }
        }
    }
}

/// Marker struct used to mark pickable entities for which you don't want to trigger a deselection
/// event when picked. This is useful for gizmos or other pickable UI entities.
#[derive(Component, Debug, Copy, Clone)]
pub struct NoDeselect;

pub fn send_selection_events(
    mut pointer_down: EventReader<output::PointerDown>,
    mut pointer_click: EventReader<output::PointerClick>,
    pointers: Query<(&PointerId, &PointerMultiselect)>,
    no_deselect: Query<&NoDeselect>,
    selectables: Query<(Entity, &PickSelection)>,
    mut selection_events: EventWriter<PointerSelectionEvent>,
) {
    for down_event in pointer_down.iter() {
        let multiselect = pointers
            .iter()
            .find_map(|(id, multi)| id.eq(&down_event.id()).then_some(multi.is_pressed))
            .unwrap_or(false);
        let target_should_deselect = !no_deselect.get(down_event.target()).is_ok();
        // Deselect everything
        if !multiselect && target_should_deselect {
            for (entity, selection) in selectables.iter() {
                if selection.is_selected {
                    selection_events.send(PointerSelectionEvent::JustDeselected(entity))
                }
            }
        }
    }

    for click_event in pointer_click.iter() {
        let multiselect = pointers
            .iter()
            .find_map(|(id, multi)| id.eq(&click_event.id()).then_some(multi.is_pressed))
            .unwrap_or(false);
        if let Ok((entity, selection)) = selectables.get(click_event.target()) {
            if multiselect {
                match selection.is_selected {
                    true => selection_events.send(PointerSelectionEvent::JustDeselected(entity)),
                    false => selection_events.send(PointerSelectionEvent::JustSelected(entity)),
                }
            } else if !selection.is_selected {
                selection_events.send(PointerSelectionEvent::JustSelected(entity))
            }
        }
    }
}
