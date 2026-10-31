use crate::input::{PlayerInput, PointerRole, TouchAction};
use std::{cell::RefCell, collections::HashSet, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use winit::keyboard::KeyCode;

pub(crate) struct BrowserInput {
    pub(crate) touch: Rc<RefCell<PlayerInput>>,
    pub(crate) keys: Rc<RefCell<HashSet<KeyCode>>>,
    pub(crate) key_presses: Rc<RefCell<HashSet<KeyCode>>>,
    listeners: Vec<(
        web_sys::EventTarget,
        &'static str,
        Closure<dyn FnMut(web_sys::Event)>,
    )>,
}
impl BrowserInput {
    pub(crate) fn new() -> Result<Self, JsValue> {
        let mut input = Self {
            touch: Rc::default(),
            keys: Rc::default(),
            key_presses: Rc::default(),
            listeners: Vec::new(),
        };
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        for kind in [
            "pointerdown",
            "pointermove",
            "pointerup",
            "pointercancel",
            "lostpointercapture",
        ] {
            let touch = input.touch.clone();
            input.listen(document.clone().into(), kind, move |event| {
                let Some(event) = event.dyn_ref::<web_sys::PointerEvent>() else {
                    return;
                };
                if event.pointer_type() == "mouse" {
                    return;
                }
                let id = event.pointer_id();
                match kind {
                    "pointerdown" => {
                        let Some(target) = event
                            .target()
                            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        else {
                            return;
                        };
                        let button = target.closest("[data-touch-action]").ok().flatten();
                        let role = if let Some(button) = &button {
                            button
                                .get_attribute("data-touch-action")
                                .and_then(|a| TouchAction::parse(&a))
                                .map(PointerRole::Button)
                        } else if target.id() == "horizon-canvas" {
                            Some(PointerRole::Look {
                                x: event.client_x() as f64,
                                y: event.client_y() as f64,
                            })
                        } else {
                            None
                        };
                        if let Some(role) = role {
                            event.prevent_default();
                            let owner = button.as_ref().unwrap_or(&target);
                            let _ = owner.set_pointer_capture(id);
                            touch.borrow_mut().press(id, role, event.time_stamp());
                        }
                    }
                    "pointermove" => touch.borrow_mut().move_pointer(
                        id,
                        event.client_x() as f64,
                        event.client_y() as f64,
                    ),
                    _ => touch
                        .borrow_mut()
                        .release(id, kind != "pointerup", event.time_stamp()),
                }
            })?;
        }
        for (target, kind) in [
            (window.into(), "blur"),
            (document.into(), "visibilitychange"),
        ] {
            let touch = input.touch.clone();
            let keys = input.keys.clone();
            let key_presses = input.key_presses.clone();
            input.listen(target, kind, move |_| {
                touch.borrow_mut().reset();
                keys.borrow_mut().clear();
                key_presses.borrow_mut().clear();
            })?;
        }
        Ok(input)
    }
    fn listen(
        &mut self,
        target: web_sys::EventTarget,
        kind: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Result<(), JsValue> {
        let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(web_sys::Event)>);
        target.add_event_listener_with_callback(kind, handler.as_ref().unchecked_ref())?;
        self.listeners.push((target, kind, handler));
        Ok(())
    }
    pub(crate) fn clear(&self) {
        self.touch.borrow_mut().reset();
        self.keys.borrow_mut().clear();
        self.key_presses.borrow_mut().clear();
    }
    pub(crate) fn key(&self, key: KeyCode, pressed: bool, repeat: bool) {
        if pressed {
            self.keys.borrow_mut().insert(key);
            if !repeat {
                self.key_presses.borrow_mut().insert(key);
            }
        } else {
            self.keys.borrow_mut().remove(&key);
        }
        if key == KeyCode::Space {
            self.touch.borrow_mut().jump_key(
                pressed,
                web_sys::window().unwrap().performance().unwrap().now(),
            );
        }
    }
}
impl Drop for BrowserInput {
    fn drop(&mut self) {
        for (target, kind, handler) in &self.listeners {
            let _ =
                target.remove_event_listener_with_callback(kind, handler.as_ref().unchecked_ref());
        }
    }
}
