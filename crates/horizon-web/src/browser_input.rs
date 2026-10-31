use crate::input::{PointerRole, TouchAction, TouchInput};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};

pub(crate) struct BrowserInput {
    pub(crate) touch: Rc<RefCell<TouchInput>>,
    pub(crate) reset: Rc<RefCell<bool>>,
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
            reset: Rc::default(),
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
                            touch.borrow_mut().press(id, role);
                        }
                    }
                    "pointermove" => touch.borrow_mut().move_pointer(
                        id,
                        event.client_x() as f64,
                        event.client_y() as f64,
                    ),
                    _ => touch.borrow_mut().release(id, kind != "pointerup"),
                }
            })?;
        }
        for (target, kind) in [
            (window.into(), "blur"),
            (document.into(), "visibilitychange"),
        ] {
            let touch = input.touch.clone();
            let reset = input.reset.clone();
            input.listen(target, kind, move |_| {
                touch.borrow_mut().reset();
                *reset.borrow_mut() = true;
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
