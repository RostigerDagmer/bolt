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

use std::{ops::Drop, sync::Arc, future::Future, borrow::BorrowMut, pin::Pin};
use std::time::{Duration, SystemTime};
use parking_lot::Mutex;

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
    pub fn build<T, W: WindowEventHandler<T>>(setup: SetupFn<T>) -> AppBuilder<T, W> {
        AppBuilder {
            prepare: None,
            setup,
            update: None,
            window_event: None,
            render: None,
        }
    }

    pub async fn new(settings: AppSettings) -> Self {
        println!("ay");
        let mut window = Window::new(
            settings.resolution[0],
            settings.resolution[1],
            settings.name.clone(),
        ).await;
        println!("Window created");
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

pub type PrepareFn = fn() -> Pin<Box<dyn futures::Future<Output = AppSettings>>>;
pub type SetupFn<T> = fn(&mut App) -> Pin<Box<dyn futures::Future<Output = T> + '_>>; // TODO: how do we specify FnOnce here?
pub type UpdateFn<T> = fn(&mut App, &mut T);
pub type RenderFn<T> = fn(Arc<Mutex<App>>, Arc<Mutex<T>>) -> Pin<Box<dyn futures::Future<Output = Result<(), AppRenderError>>>>;
pub trait WindowEventHandler<T> {
    async fn window_event<'a, 'b>(&self, data: &'a mut T, target: &'b async_winit::window::Window<ThreadUnsafe>);
}

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
pub struct AppBuilder<T: 'static, W: WindowEventHandler<T> + 'static> {
    pub prepare: Option<PrepareFn>,
    pub setup: SetupFn<T>,
    pub update: Option<UpdateFn<T>>,
    pub window_event: Option<W>,
    pub render: Option<RenderFn<T>>,
}

impl<T, W: WindowEventHandler<T> + 'static> AppBuilder<T, W> {
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

    pub fn window_event(mut self, window_event: W) -> Self {
        self.window_event = Some(window_event);
        self
    }

    pub async fn run(self) {
        println!("Running app");
        main_loop(self).await;
    }
}

async fn main_loop<T: 'static, W: WindowEventHandler<T>>(builder: AppBuilder<T, W>) {
    let event_loop = EventLoop::<ThreadUnsafe>::new();
    let mut settings = AppSettings::default();
    match builder.prepare {
        Some(prepare) => {
            settings = prepare().await;
        }
        None => {}
    }
    let window_target = event_loop.window_target().clone();
    
    event_loop.block_on(async move {
        let mut app = App::new(settings).await;
        let mut app_data = (builder.setup)(&mut app).await;
        let mut dirty_swapchain = false;
        let now = SystemTime::now();
        let mut modifiers = ModifiersState::default();
        let render_fn = builder.render.unwrap();
        let mut app_data = Arc::new(Mutex::new(app_data));
        let window = app.window.handle().clone();
        let device = app.renderer.context.device().clone();
        let mut app = Arc::new(Mutex::new(app));
        loop {
            println!("rendarin da renda loop");
            if !window.is_minimized().await.unwrap() {
                println!("not minimized");
                // window_target.resumed().await;
                println!("here");
                if dirty_swapchain {
                    app.lock().recreate_swapchain();
                    dirty_swapchain = false;
                }
                match (builder.window_event) {
                    Some(ref handler) => {
                        let mut guard = app_data.lock();
                        handler.window_event(guard.borrow_mut(), &window);
                    }
                    None => {}
                }
                // window.redraw_requested().await;
                println!("here2");
                let now = now.elapsed().unwrap();
                if app.lock().elapsed_ticks % 10 == 0 {
                    let mut guard = app.lock();
                    let cpu_time = now.as_millis() as f32 - guard.elapsed_time.as_millis() as f32;
                    let title = format!("{} | cpu:{:.1} ms, gpu:{:.1} ms", guard.settings.name, cpu_time, guard.renderer.gpu_frame_time);
                    guard.window.set_title(&title);
                }
                app.lock().elapsed_time = now;

                match (builder.update) {
                    Some(update_fn) => {
                        let mut guard = app.lock();
                        let mut data_guard = app_data.lock();
                        update_fn(guard.borrow_mut(), data_guard.borrow_mut());
                    }
                    None => {}
                }

                dirty_swapchain = {
                    // Some(render_fn) => {
                        println!("here3");
                        let future = render_fn(app.clone(), app_data.clone()).await;
                        println!("here4");
                        matches!(
                            future,
                            Err(AppRenderError::DirtySwapchain)
                        )
                    // }
                    // None => false,
                };
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
    println!("exiting")
}
