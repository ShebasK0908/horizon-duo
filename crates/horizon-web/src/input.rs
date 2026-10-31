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
pub(crate) struct PlayerInput {
    pointers: HashMap<i32, PointerRole>,
    presses: HashSet<TouchAction>,
    pub(crate) actions: VecDeque<TouchAction>,
    look: [f32; 2],
    jump_sources: HashSet<JumpSource>,
    sampled_jump_sources: HashSet<JumpSource>,
    jumps: VecDeque<JumpTransition>,
    last_jump_press: Option<f64>,
}
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum JumpSource {
    Keyboard,
    Pointer(i32),
}
struct JumpTransition {
    source: JumpSource,
    pressed: bool,
    at_ms: f64,
}
pub(crate) struct JumpSample {
    pub(crate) held: bool,
    pub(crate) new_sequence: bool,
}
impl PlayerInput {
    pub(crate) fn press(&mut self, id: i32, role: PointerRole, at_ms: f64) {
        if self.pointers.contains_key(&id) {
            return;
        }
        if let PointerRole::Button(action) = role {
            match action {
                TouchAction::Place | TouchAction::Break => self.actions.push_back(action),
                TouchAction::Jump => self.set_jump(JumpSource::Pointer(id), true, at_ms),
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
    pub(crate) fn release(&mut self, id: i32, cancelled: bool, at_ms: f64) {
        if let Some(PointerRole::Button(action)) = self.pointers.remove(&id) {
            if action == TouchAction::Jump {
                let source = JumpSource::Pointer(id);
                if cancelled
                    && let Some(index) = self
                        .jumps
                        .iter()
                        .rposition(|event| event.source == source && event.pressed)
                {
                    self.jumps.remove(index);
                }
                self.set_jump(source, false, at_ms);
            }
            if cancelled
                && !self
                    .pointers
                    .values()
                    .any(|role| matches!(role, PointerRole::Button(a) if *a == action))
            {
                self.presses.remove(&action);
            }
        }
    }
    fn set_jump(&mut self, source: JumpSource, pressed: bool, at_ms: f64) {
        let changed = if pressed {
            self.jump_sources.insert(source)
        } else {
            self.jump_sources.remove(&source)
        };
        if changed {
            self.jumps.push_back(JumpTransition {
                source,
                pressed,
                at_ms,
            });
        }
    }
    pub(crate) fn jump_key(&mut self, pressed: bool, at_ms: f64) {
        self.set_jump(JumpSource::Keyboard, pressed, at_ms);
    }
    pub(crate) fn jump(&mut self) -> JumpSample {
        while let Some(event) = self.jumps.pop_front() {
            let was_held = !self.sampled_jump_sources.is_empty();
            if event.pressed {
                self.sampled_jump_sources.insert(event.source);
            } else {
                self.sampled_jump_sources.remove(&event.source);
            }
            let held = !self.sampled_jump_sources.is_empty();
            if held != was_held {
                return JumpSample {
                    held,
                    new_sequence: held
                        && self
                            .last_jump_press
                            .replace(event.at_ms)
                            .is_none_or(|last| event.at_ms - last >= 350.),
                };
            }
        }
        JumpSample {
            held: !self.sampled_jump_sources.is_empty(),
            new_sequence: false,
        }
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
        let mut input = PlayerInput::default();
        input.press(1, PointerRole::Button(TouchAction::Jump), 0.);
        input.release(1, false, 1.);
        assert!(input.jump().held);
        input.finish_tick();
        assert!(!input.jump().held);
    }
    #[test]
    fn owners_release_independently_and_look_does_not_steal_movement() {
        let mut input = PlayerInput::default();
        for id in [1, 2] {
            input.press(id, PointerRole::Button(TouchAction::Forward), 0.);
        }
        input.press(3, PointerRole::Look { x: 5., y: 10. }, 0.);
        input.move_pointer(3, 15., 8.);
        input.release(1, true, 1.);
        input.finish_tick();
        assert!(input.held(TouchAction::Forward));
        assert_eq!(input.take_look(), [10., -2.]);
        input.release(2, true, 2.);
        assert!(!input.held(TouchAction::Forward));
    }
    #[test]
    fn actions_retain_each_press_and_reset_clears_every_source() {
        let mut input = PlayerInput::default();
        for id in [1, 2] {
            input.press(id, PointerRole::Button(TouchAction::Place), 0.);
            input.release(id, false, 1.);
        }
        assert_eq!(input.actions.len(), 2);
        input.press(3, PointerRole::Button(TouchAction::Left), 0.);
        input.reset();
        assert!(input.actions.is_empty());
        assert!(!input.held(TouchAction::Left));
    }
    #[test]
    fn retained_jump_presses_preserve_their_real_time_gap() {
        for (gap, new_sequence) in [(100., false), (350., true), (720., true)] {
            let mut input = PlayerInput::default();
            input.press(1, PointerRole::Button(TouchAction::Jump), 0.);
            input.release(1, false, 1.);
            input.press(1, PointerRole::Button(TouchAction::Jump), gap);
            input.release(1, false, gap + 1.);
            assert!(input.jump().new_sequence);
            assert!(!input.jump().held);
            assert_eq!(input.jump().new_sequence, new_sequence);
            assert!(!input.jump().held);
        }
    }
    #[test]
    fn keyboard_and_touch_share_jump_ownership_and_timing() {
        let mut input = PlayerInput::default();
        input.jump_key(true, 0.);
        input.press(1, PointerRole::Button(TouchAction::Jump), 10.);
        input.jump_key(false, 20.);
        assert!(input.jump().held);
        assert!(input.jump().held);
        input.release(1, false, 30.);
        assert!(!input.jump().held);
        input.jump_key(true, 100.);
        assert!(!input.jump().new_sequence);
        input.reset();
        input.jump_key(true, 110.);
        assert!(input.jump().new_sequence);
    }
    #[test]
    fn cancelling_jump_preserves_completed_keyboard_taps_before_and_during_touch() {
        for keyboard_first in [true, false] {
            let mut input = PlayerInput::default();
            if !keyboard_first {
                input.press(1, PointerRole::Button(TouchAction::Jump), 0.);
            }
            input.jump_key(true, 1.);
            input.jump_key(false, 2.);
            if keyboard_first {
                input.press(1, PointerRole::Button(TouchAction::Jump), 3.);
            }
            input.release(1, true, 4.);
            assert!(input.jump().held);
            assert!(!input.jump().held);
            assert!(!input.jump().held);
        }
    }
    #[test]
    fn cancellation_preserves_completed_contacts_with_reused_pointer_ids() {
        for role in [
            PointerRole::Button(TouchAction::Jump),
            PointerRole::Look { x: 0., y: 0. },
        ] {
            let mut input = PlayerInput::default();
            input.press(1, PointerRole::Button(TouchAction::Jump), 0.);
            input.release(1, false, 1.);
            input.press(1, role, 2.);
            input.release(1, true, 3.);
            assert!(input.jump().held);
            assert!(!input.jump().held);
            assert!(!input.jump().held);
        }
    }
    #[test]
    fn cancellation_removes_only_a_pending_contact_and_releases_sampled_owners() {
        let mut input = PlayerInput::default();
        input.press(1, PointerRole::Button(TouchAction::Jump), 0.);
        input.release(1, true, 1.);
        assert!(!input.jump().held);
        input.press(1, PointerRole::Button(TouchAction::Jump), 2.);
        assert!(input.jump().held);
        input.jump_key(true, 3.);
        input.release(1, true, 4.);
        assert!(input.jump().held);
        input.jump_key(false, 5.);
        assert!(!input.jump().held);
    }
}
