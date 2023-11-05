#![allow(dead_code)]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]

extern crate num_cpus;
// use resource::{BindlessManager, ResourceManager, ManagerBaseSettings};
use winit::{
    event::{ElementState, Event, ModifiersState, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget}, ThreadUnsafe,
};
use futures_lite::{future::FutureExt, pin};

use std::{ops::Drop, sync::{Arc, Mutex}, future::Future, borrow::BorrowMut, pin::Pin};
use std::time::{Duration, SystemTime};

mod buffer;
mod context;
mod descriptor;
pub mod pipeline;
mod pools;
pub mod prelude;
mod renderer;
mod renderpass;
pub mod scene;
mod swapchain;
mod texture;
pub mod util;
mod window;
pub mod ray;
pub mod sim;
pub mod resource;
pub mod debug;
pub mod compute_pipeline;
pub mod compute_pass;
pub mod sort;
pub mod command;

pub use crate::buffer::*;
pub use crate::context::*;
pub use crate::descriptor::*;
pub use crate::pipeline::*;
pub use crate::pools::*;
pub use crate::renderer::*;
pub use crate::renderpass::*;
pub use crate::swapchain::*;
pub use crate::texture::*;
pub use crate::window::*;
pub use crate::compute_pipeline::*;
pub use crate::sort::*;
pub use crate::compute_pass::*;
pub use crate::command::*;
pub use ash;
pub use glam;
pub use async_winit as winit;

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}

pub trait Resource<T> {
    fn handle(&self) -> T;
    fn handles(&self) -> Option<&Vec<T>> {
        None
    }
}

pub trait Vertex {
    fn stride() -> u32;
    fn format_offset() -> Vec<(ash::vk::Format, u32)>;
}

pub struct App {
    pub settings: AppSettings,
    pub renderer: AppRenderer,
    // TODO: make this trait bound
    //pub resource_manager: BindlessManager,
    pub window: Window,
    pub elapsed_time: Duration,
    pub elapsed_ticks: u64,
}

impl App {
    pub fn build<T>(setup: SetupFn<T>) -> AppBuilder<T> {
        AppBuilder {
            prepare: None,
            setup,
            update: None,
            window_event: None,
            render: None,
        }
    }

    pub async fn new(settings: AppSettings) -> Self {
        let mut window = Window::new(
            settings.resolution[0],
            settings.resolution[1],
            settings.name.clone(),
        ).await;
        // create the context

        let (shared_context, queue_manager) = create_shared_context_and_queue_manager(&mut window, &settings.render.clone());

        //let resource_manager = BindlessManager::new(&mut window, &settings.manager, Some(settings.render.clone()));
        let renderer = AppRenderer::new(&mut window, shared_context.clone(), queue_manager, settings.render.clone()).await;
        App {
            settings,
            renderer,
            //resource_manager,
            window,
            elapsed_time: Duration::default(),
            elapsed_ticks: 0,
        }
    }

    pub fn recreate_swapchain(&mut self) {
        self.renderer.recreate_swapchain(&self.window);
    }
}

pub type PrepareFn = fn() -> AppSettings;
pub type SetupFn<T> = fn(&mut App) -> T; // TODO: how do we specify FnOnce here?
pub type UpdateFn<T> = fn(&mut App, &mut T);
pub type RenderFn<T> = fn(&mut App, &mut T) -> Pin<Box<dyn futures::Future<Output = Result<(), AppRenderError>>>>;
pub type WindowEventFn<T> = fn(&mut App, &mut T, target: &EventLoopWindowTarget);

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub name: String,
    pub resolution: [u32; 2],
    pub render: RendererSettings,
    //pub manager: ManagerBaseSettings,
    
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            name: "App".to_string(),
            resolution: [1280, 720],
            render: RendererSettings::default(),
            //manager: ManagerBaseSettings::default(),
        }
    }
}
pub struct AppBuilder<T: 'static> {
    pub prepare: Option<PrepareFn>,
    pub setup: SetupFn<T>,
    pub update: Option<UpdateFn<T>>,
    pub window_event: Option<WindowEventFn<T>>,
    pub render: Option<RenderFn<T>>,
}

impl<T> AppBuilder<T> {
    pub fn prepare(mut self, prepare: PrepareFn) -> Self {
        self.prepare = Some(prepare);
        self
    }

    pub fn update(mut self, update: UpdateFn<T>) -> Self {
        self.update = Some(update);
        self
    }

    pub fn render(mut self, render: RenderFn<T>) -> Self {
        self.render = Some(render);
        self
    }

    pub fn window_event(mut self, window_event: WindowEventFn<T>) -> Self {
        self.window_event = Some(window_event);
        self
    }

    pub fn run(self) {
        main_loop(self);
    }
}

async fn main_loop<T: 'static>(builder: AppBuilder<T>) {
    let event_loop = EventLoop::<ThreadUnsafe>::new();
    let mut settings = AppSettings::default();
    match builder.prepare {
        Some(prepare) => {
            settings = prepare();
        }
        None => {}
    }
    let mut app = App::new(settings).await;
    let mut app_data = (builder.setup)(&mut app);
    let mut dirty_swapchain = false;
    

    let now = SystemTime::now();
    let mut modifiers = ModifiersState::default();
    let window_target = event_loop.window_target().clone();

    event_loop.block_on(async move {
        loop {
            let window = app.window.handle().clone();
            let device = app.renderer.context.device().clone();
            if !window.is_minimized().await.unwrap() {
                window_target.resumed().await;
                if dirty_swapchain {
                    app.recreate_swapchain();
                    dirty_swapchain = false;
                }

                
                let close = async {
                    window.close_requested().wait().await;
                    true
                };
                let close_keyboard = async {
                    let input = window.keyboard_input().await.input;
                    if input.state == ElementState::Pressed {
                        if input.virtual_keycode == Some(VirtualKeyCode::Q)
                            && (modifiers.ctrl() || modifiers.logo())
                        {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };
                let mouse = async {
                    let input = window.mouse_input().await;
                    if input.state == ElementState::Pressed {
                        true
                    } else {
                        false
                    }
                };

                let m = async {
                    let modifiers = window.modifiers_changed().await;
                    modifiers
                };

                match (builder.window_event) {
                    Some(event_fn) => {
                        event_fn(&mut app, &mut app_data, &window_target);
                    }
                    None => {}
                }

                let main_events_cleared = async {
                    window.redraw_requested().await;
                    let now = now.elapsed().unwrap();
                    if app.elapsed_ticks % 10 == 0 {
                        let cpu_time = now.as_millis() as f32 - app.elapsed_time.as_millis() as f32;
                        let title = format!("{} | cpu:{:.1} ms, gpu:{:.1} ms", app.settings.name, cpu_time, app.renderer.gpu_frame_time);
                        app.window.set_title(&title);
                    }
                    app.elapsed_time = now;

                    match (builder.update) {
                        Some(update_fn) => {
                            update_fn(&mut app, &mut app_data);
                        }
                        None => {}
                    }

                    dirty_swapchain = match (builder.render) {
                        Some(render_fn) => {
                            let future = render_fn(&mut app, &mut app_data);
                            let res = future.await;
                            matches!(
                                res,
                                Err(AppRenderError::DirtySwapchain)
                            )
                        }
                        None => false,
                    };
                };

                let suspend = async {
                    window_target.suspended().wait().await;
                    false
                };

                let destroyed = async {
                    window.destroyed().await;
                    true
                };

                if destroyed.await {
                    unsafe {
                        device.device_wait_idle().unwrap();
                    }
                }

                let needs_exit = close.or(close_keyboard).or(suspend).await;

                if needs_exit {
                    window_target.exit().await;
                }

            }
        }
    });
}
