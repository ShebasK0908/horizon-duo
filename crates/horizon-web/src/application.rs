use crate::{client::BrowserClient, fixture::RenderFixture, resources::WorldResources};
use horizon_client_render::{assets::RenderAssets, mesher::SectionMesh};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::{Window, WindowId},
};

pub(crate) enum Startup {
    World {
        resources: Box<WorldResources>,
        bridge: String,
        world_id: String,
        name: String,
    },
    Fixture {
        render: Option<RenderAssets>,
        mesh: SectionMesh,
    },
}
pub(crate) enum Running {
    World(Box<BrowserClient>),
    Fixture(Box<RenderFixture>),
}
impl Running {
    fn handler(&mut self) -> &mut dyn ApplicationHandler<Ready> {
        match self {
            Self::World(client) => client.as_mut(),
            Self::Fixture(fixture) => fixture.as_mut(),
        }
    }
}
pub(crate) type Ready = Result<Running, String>;
struct BrowserApplication {
    startup: Option<Startup>,
    proxy: EventLoopProxy<Ready>,
    running: Option<Running>,
    window: Option<Arc<Window>>,
}
pub(crate) fn start(startup: Startup) -> Result<(), JsValue> {
    let events = EventLoop::<Ready>::with_user_event()
        .build()
        .map_err(error)?;
    let proxy = events.create_proxy();
    events.spawn_app(BrowserApplication {
        startup: Some(startup),
        proxy,
        running: None,
        window: None,
    });
    Ok(())
}
impl ApplicationHandler<Ready> for BrowserApplication {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        let Some(startup) = self.startup.take() else {
            return;
        };
        let result = (|| {
            let window = Arc::new(
                events
                    .create_window(Window::default_attributes().with_title("Horizon"))
                    .map_err(error)?,
            );
            let document = web_sys::window().unwrap().document().unwrap();
            let canvas = window.canvas().ok_or_else(|| error("No browser canvas"))?;
            canvas.set_id("horizon-canvas");
            document
                .get_element_by_id("viewport")
                .unwrap()
                .append_child(&canvas)?;
            Ok::<_, JsValue>(window)
        })();
        let window = match result {
            Ok(window) => window,
            Err(failure) => {
                show_failure(&format!("{failure:?}"));
                events.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        let proxy = self.proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = match startup {
                Startup::World {
                    resources,
                    bridge,
                    world_id,
                    name,
                } => BrowserClient::new(window, *resources, bridge, world_id, name)
                    .await
                    .map(|client| Running::World(Box::new(client))),
                Startup::Fixture { render, mesh } => RenderFixture::new(window, render, mesh)
                    .await
                    .map(|fixture| Running::Fixture(Box::new(fixture))),
            };
            let _ = proxy.send_event(result.map_err(|error| format!("{error:?}")));
        });
    }
    fn user_event(&mut self, events: &ActiveEventLoop, ready: Ready) {
        match ready {
            Ok(mut running) => {
                if let Some(window) = &self.window {
                    running.handler().window_event(
                        events,
                        window.id(),
                        WindowEvent::Resized(window.inner_size()),
                    );
                    window.request_redraw();
                }
                self.running = Some(running);
            }
            Err(error) => {
                show_failure(&error);
                events.exit();
            }
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(running) = &mut self.running {
            running.handler().window_event(events, id, event);
        }
    }
    fn device_event(&mut self, events: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        if let Some(running) = &mut self.running {
            running.handler().device_event(events, id, event);
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if let Some(running) = &mut self.running {
            running.handler().about_to_wait(events);
        }
    }
}
fn error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
fn show_failure(error: &str) {
    log::error!("{error}");
    if let Some(element) = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("error")
    {
        element.set_text_content(Some("Horizon could not start graphics. Enable WebGPU or try an updated browser, then reload."));
    }
}
