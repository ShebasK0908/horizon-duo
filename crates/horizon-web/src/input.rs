use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TouchAction {
    Forward,
    Backward,
    Left,
    Right,
    Jump,
    Descend,
    Place,
    Break,
}
impl TouchAction {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "forward" => Self::Forward,
            "backward" => Self::Backward,
            "left" => Self::Left,
            "right" => Self::Right,
            "jump" => Self::Jump,
            "descend" => Self::Descend,
            "place" => Self::Place,
            "break" => Self::Break,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) enum PointerRole {
    Button(TouchAction),
    Look { x: f64, y: f64 },
}
#[derive(Default)]
pub(crate) struct TouchInput {
    pointers: HashMap<i32, PointerRole>,
    presses: HashSet<TouchAction>,
    pub(crate) actions: VecDeque<TouchAction>,
    look: [f32; 2],
    jumps: VecDeque<bool>,
}
impl TouchInput {
    pub(crate) fn press(&mut self, id: i32, role: PointerRole) {
        if self.pointers.contains_key(&id) {
            return;
        }
        if let PointerRole::Button(action) = role {
            match action {
                TouchAction::Place | TouchAction::Break => self.actions.push_back(action),
                TouchAction::Jump => self.jumps.push_back(true),
                _ => {
                    self.presses.insert(action);
                }
            }
        } else if self
            .pointers
            .values()
            .any(|role| matches!(role, PointerRole::Look { .. }))
        {
            return;
        }
        self.pointers.insert(id, role);
    }
    pub(crate) fn move_pointer(&mut self, id: i32, x: f64, y: f64) {
        if let Some(PointerRole::Look { x: old_x, y: old_y }) = self.pointers.get_mut(&id) {
            self.look[0] += (x - *old_x) as f32;
            self.look[1] += (y - *old_y) as f32;
            *old_x = x;
            *old_y = y;
        }
    }
    pub(crate) fn release(&mut self, id: i32, cancelled: bool) {
        if let Some(PointerRole::Button(action)) = self.pointers.remove(&id)
            && !self
                .pointers
                .values()
                .any(|role| matches!(role, PointerRole::Button(a) if *a == action))
        {
            if action == TouchAction::Jump {
                if cancelled {
                    self.jumps.clear();
                } else {
                    self.jumps.push_back(false);
                }
            }
            if cancelled {
                self.presses.remove(&action);
            }
        }
    }
    pub(crate) fn jump(&mut self) -> bool {
        self.jumps
            .pop_front()
            .unwrap_or_else(|| self.held(TouchAction::Jump))
    }
    pub(crate) fn held(&self, action: TouchAction) -> bool {
        self.presses.contains(&action)
            || self
                .pointers
                .values()
                .any(|role| matches!(role, PointerRole::Button(a) if *a == action))
    }
    pub(crate) fn finish_tick(&mut self) {
        self.presses.clear();
    }
    pub(crate) fn take_look(&mut self) -> [f32; 2] {
        std::mem::take(&mut self.look)
    }
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_names_accept_only_known_actions() {
        for name in [
            "forward", "backward", "left", "right", "jump", "descend", "place", "break",
        ] {
            assert!(TouchAction::parse(name).is_some());
        }
        assert_eq!(TouchAction::parse("unknown"), None);
    }
    #[test]
    fn quick_tap_survives_until_tick_and_releases() {
        let mut input = TouchInput::default();
        input.press(1, PointerRole::Button(TouchAction::Jump));
        input.release(1, false);
        assert!(input.jump());
        input.finish_tick();
        assert!(!input.jump());
    }
    #[test]
    fn owners_release_independently_and_look_does_not_steal_movement() {
        let mut input = TouchInput::default();
        for id in [1, 2] {
            input.press(id, PointerRole::Button(TouchAction::Forward));
        }
        input.press(3, PointerRole::Look { x: 5., y: 10. });
        input.move_pointer(3, 15., 8.);
        input.release(1, true);
        input.finish_tick();
        assert!(input.held(TouchAction::Forward));
        assert_eq!(input.take_look(), [10., -2.]);
        input.release(2, true);
        assert!(!input.held(TouchAction::Forward));
    }
    #[test]
    fn actions_retain_each_press_and_reset_clears_every_source() {
        let mut input = TouchInput::default();
        for id in [1, 2] {
            input.press(id, PointerRole::Button(TouchAction::Place));
            input.release(id, false);
        }
        assert_eq!(input.actions.len(), 2);
        input.press(3, PointerRole::Button(TouchAction::Left));
        input.reset();
        assert!(input.actions.is_empty());
        assert!(!input.held(TouchAction::Left));
    }
}
