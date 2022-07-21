use std::fmt::Debug;

use crate::PointerId;
use bevy::{prelude::*, render::camera::RenderTarget};

/// Tracks the state of the pointer's buttons in response to [`InputPress`]s.
#[derive(Debug, Default, Clone, Component, PartialEq)]
pub struct PointerPress {
    primary: bool,
    secondary: bool,
    middle: bool,
}
impl PointerPress {
    #[inline]
    pub fn is_primary_down(&self) -> bool {
        self.primary
    }
    #[inline]
    pub fn is_secondary_down(&self) -> bool {
        self.secondary
    }
    #[inline]
    pub fn is_middle_down(&self) -> bool {
        self.middle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressStage {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPress {
    pub id: PointerId,
    pub press: PressStage,
    pub button: PointerButton,
}
impl InputPress {
    pub fn new_down(id: PointerId, button: PointerButton) -> InputPress {
        Self {
            id,
            press: PressStage::Down,
            button,
        }
    }

    pub fn new_up(id: PointerId, button: PointerButton) -> InputPress {
        Self {
            id,
            press: PressStage::Up,
            button,
        }
    }

    #[inline]
    pub fn is_just_down(&self, id: &PointerId, button: PointerButton) -> bool {
        *self == Self::new_down(*id, button)
    }

    #[inline]
    pub fn is_just_up(&self, id: &PointerId, button: PointerButton) -> bool {
        *self == Self::new_up(*id, button)
    }

    pub fn receive(
        mut events: EventReader<InputPress>,
        mut pointers: Query<(&PointerId, &mut PointerPress)>,
    ) {
        for press_event in events.iter() {
            pointers.for_each_mut(|(pointer_id, mut pointer)| {
                if *pointer_id == press_event.id {
                    let new_value = press_event.press == PressStage::Down;
                    match press_event.button {
                        PointerButton::Primary => pointer.primary = new_value,
                        PointerButton::Secondary => pointer.secondary = new_value,
                        PointerButton::Middle => pointer.middle = new_value,
                    }
                }
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

/// Represents an input pointer used for picking.
#[derive(Debug, Default, Clone, Component, PartialEq)]
pub struct PointerPosition {
    location: Option<Location>,
}
impl PointerPosition {
    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct InputMove {
    pub id: PointerId,
    pub location: Location,
}
impl InputMove {
    pub fn new(id: PointerId, location: Location) -> InputMove {
        Self { id, location }
    }

    pub fn receive(
        mut events: EventReader<InputMove>,
        mut pointers: Query<(&PointerId, &mut PointerPosition)>,
    ) {
        for event_pointer in events.iter() {
            pointers.for_each_mut(|(id, mut pointer)| {
                if *id == event_pointer.id {
                    pointer.location = Some(event_pointer.location.to_owned());
                }
            })
        }
    }
}

#[derive(Debug, Clone, Component, PartialEq)]
pub struct Location {
    pub target: RenderTarget,
    pub position: Vec2,
}
impl Location {
    #[inline]
    pub fn is_in_viewport(&self, camera: &Camera) -> bool {
        camera
            .logical_viewport_rect()
            .map(|(min, max)| {
                (self.position - min).min_element() >= 0.0
                    && (self.position - max).max_element() <= 0.0
            })
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_same_target(&self, camera: &Camera) -> bool {
        self.target == camera.target
    }
}
