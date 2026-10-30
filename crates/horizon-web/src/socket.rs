use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CloseEvent, MessageEvent, WebSocket};
const MAX_QUEUED: usize = 16 << 20;
#[derive(Default)]
struct Inbox {
    bytes: usize,
    messages: VecDeque<Vec<u8>>,
    error: Option<String>,
}
pub struct ClientConnection {
    socket: WebSocket,
    inbox: Rc<RefCell<Inbox>>,
    outgoing: VecDeque<Vec<u8>>,
    queued: usize,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _close: Closure<dyn FnMut(CloseEvent)>,
}
impl ClientConnection {
    pub fn connect(url: &str) -> Result<Self, JsValue> {
        let socket = WebSocket::new(url)?;
        socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let inbox = Rc::new(RefCell::new(Inbox::default()));
        let shared = inbox.clone();
        let transport = socket.clone();
        let message = Closure::new(move |event: MessageEvent| {
            let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() else {
                shared.borrow_mut().error =
                    Some("The world bridge sent a nonbinary message".into());
                let _ = transport.close();
                return;
            };
            let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
            let mut inbox = shared.borrow_mut();
            if inbox.bytes.saturating_add(bytes.len()) > MAX_QUEUED {
                inbox.error = Some("The server exceeded the browser's pending data limit".into());
                let _ = transport.close();
                return;
            }
            inbox.bytes += bytes.len();
            inbox.messages.push_back(bytes);
        });
        let shared = inbox.clone();
        let close = Closure::new(move |event: CloseEvent| {
            shared.borrow_mut().error = Some(format!(
                "World connection closed ({}) {}",
                event.code(),
                event.reason()
            ));
        });
        socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
        socket.set_onclose(Some(close.as_ref().unchecked_ref()));
        Ok(Self {
            socket,
            inbox,
            outgoing: VecDeque::new(),
            queued: 0,
            _message: message,
            _close: close,
        })
    }
    pub fn receive(&mut self) -> Result<Option<Vec<u8>>, String> {
        let mut inbox = self.inbox.borrow_mut();
        if let Some(bytes) = inbox.messages.pop_front() {
            inbox.bytes -= bytes.len();
            return Ok(Some(bytes));
        }
        if let Some(error) = inbox.error.take() {
            return Err(error);
        }
        Ok(None)
    }
    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        if self.queued.saturating_add(bytes.len()) > MAX_QUEUED {
            return Err("The browser's outgoing packet limit was exceeded".into());
        }
        self.queued += bytes.len();
        self.outgoing.push_back(bytes);
        Ok(())
    }
    pub fn flush(&mut self) -> Result<(), String> {
        if self.socket.ready_state() != WebSocket::OPEN {
            return Ok(());
        }
        while self.socket.buffered_amount() < (1 << 20) {
            let Some(bytes) = self.outgoing.pop_front() else {
                break;
            };
            self.queued -= bytes.len();
            self.socket
                .send_with_u8_array(&bytes)
                .map_err(|e| format!("Cannot send world packet: {e:?}"))?;
        }
        Ok(())
    }
}
impl Drop for ClientConnection {
    fn drop(&mut self) {
        self.socket.set_onmessage(None);
        self.socket.set_onclose(None);
        let _ = self.socket.close();
    }
}
