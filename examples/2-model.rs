use std::env;
use std::pin::Pin;

use ash::vk::Event;
use async_winit::ThreadUnsafe;
//#![windows_subsystem = "windows"]
use bolt::prelude::*;
use bolt::scene;
use futures::Future;
use parking_lot::Mutex;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct SceneData {
    mvp: Mat4,
    normal: Mat4,
}

pub struct PerFrameData {
    pub ubo: bolt::Buffer,
    pub desc_set: bolt::DescriptorSet,
}

pub struct AppData {
    pub scene: scene::Scene,
    pub pipeline: bolt::Pipeline,
    pub desc_set_layout: bolt::DescriptorSetLayout,
    pub pipeline_layout: bolt::PipelineLayout,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,
}

pub fn setup(app: &mut bolt::App) -> Pin<Box<dyn Future<Output = AppData> + '_>> {
    Box::pin(
    async move {

        let context = &app.renderer.context;
        let index = std::env::args().position(|arg| arg == "--model").unwrap();
        let scene = scene::load_scene(
            context.clone(),
            &bolt::util::find_asset(&std::env::args().nth(index + 1).expect("no file given"))
                .unwrap(),
        );
    
        let mut desc_set_layout = bolt::DescriptorSetLayout::new(
            context.clone(),
            bolt::DescriptorSetLayoutInfo::default().binding(
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
                vk::ShaderStageFlags::ALL,
            )
        );
        let pipeline_layout = bolt::PipelineLayout::new(
            context.clone(),
            bolt::PipelineLayoutInfo::default().desc_set_layout(desc_set_layout.handle()),
        );
        let pipeline = bolt::Pipeline::new(
            context.clone(),
            bolt::PipelineInfo::default()
                .layout(pipeline_layout.handle())
                .render_pass_info(app.renderer.swapchain.get_transient_render_pass_info())
                .vert(bolt::util::find_asset("glsl/model.vert").unwrap())
                .frag(bolt::util::find_asset("glsl/model.frag").unwrap())
                .front_face(vk::FrontFace::CLOCKWISE)
                .vertex_type::<scene::ModelVertex>(),
        );
    
        let mut camera = scene::Camera::new(app.window.get_size().await);
        camera.look_at(Vec3::splat(3.0), vec3(0.0, 0.5, 0.0), -Vec3::Y);
    
        let scene_data = SceneData {
            mvp: camera.perspective_matrix() * camera.view_matrix() * scene.meshes[0].transform,
            normal: (camera.view_matrix() * scene.meshes[0].transform)
                .inverse()
                .transpose(),
        };
    
        let mut per_frame = Vec::<PerFrameData>::new();
        for _ in 0..app.renderer.get_frames_count() {
            let ubo = bolt::Buffer::from_data(
                context.clone(),
                bolt::BufferInfo::default()
                    .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                    .cpu_to_gpu(),
                &[scene_data],
            );
            let desc_set = desc_set_layout
                .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
            per_frame.push(PerFrameData { ubo, desc_set });
        }
    
        AppData {
            scene,
            pipeline,
            desc_set_layout,
            pipeline_layout,
            per_frame,
            manip: scene::CameraManip {
                camera,
                input: scene::CameraInput::default(),
                mode: scene::CameraMode::Fly,
                ..Default::default()
            },
        }
    })
}

struct WindowEventHandler {}
impl bolt::WindowEventHandler<AppData> for WindowEventHandler {
    async fn window_event<'a, 'b>(&self, data: &'a mut AppData, window: &'b async_winit::window::Window<ThreadUnsafe>) {
        data.manip.update(window).await;
    }
}

pub fn render(app: Arc<Mutex<bolt::App>>, data: Arc<Mutex<AppData>>) -> Pin<Box<dyn Future<Output = Result<(), bolt::AppRenderError>>>> {
    Box::pin(async move {
        let mut app = app.lock();
        let mut data = data.lock();
        println!("ayo I'm rendering");
        let (image_aquired_semaphore, cmd) = app.renderer.begin_frame_default()?;
        let ref camera = data.manip.camera;
        //TODO: move mesh transform in push constant?
        let scene_data = SceneData {
            mvp: camera.perspective_matrix() * camera.view_matrix() * data.scene.meshes[0].transform,
            normal: (camera.view_matrix() * data.scene.meshes[0].transform)
                .inverse()
                .transpose(),
        };
        data.per_frame[app.renderer.active_frame_index]
            .ubo
            .update(&[scene_data]);
        let pipeline_layout = data.pipeline_layout.handle();
        let descriptor_sets = [data.per_frame[app.renderer.active_frame_index].desc_set.handle()];
        let device = app.renderer.context.device();
        unsafe {
            device.cmd_set_scissor(cmd, 0, &[app.window.get_rect().await]);
            device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport().await]);
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, data.pipeline.handle());
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                descriptor_sets.as_slice(),
                &[],
            );
        }
        data.scene.vulkan_meshes.iter().for_each(|mesh| mesh.cmd_draw(cmd));
        app.renderer.end_frame_default(image_aquired_semaphore, cmd)
    })
}

pub fn prepare() -> Pin<Box<dyn Future<Output = bolt::AppSettings>>> {
    Box::pin(async {
        bolt::AppSettings {
            name: "Model App".to_string(),
            resolution: [1500, 800],
            render: bolt::RendererSettings {
                samples: 8,
                clear_color: Vec4::splat(0.15),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

#[tokio::main]
pub async fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    let event_handler = WindowEventHandler {};
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(event_handler)
        .run().await;
}
