//#![windows_subsystem = "windows"]
use bolt::prelude::*;
use bolt::ray;
use bolt::scene;
use winit::event::WindowEvent;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct SceneUniforms {
    model: Mat4,
    view: Mat4,
    view_inverse: Mat4,
    projection: Mat4,
    projection_inverse: Mat4,
    model_view_projection: Mat4,
    frame: UVec3,
}

impl SceneUniforms {
    pub fn from(camera: &scene::Camera, frame: UVec3) -> SceneUniforms {
        let vp = camera.perspective_matrix() * camera.view_matrix();
        SceneUniforms {
            model: Mat4::IDENTITY,
            view: camera.view_matrix(),
            view_inverse: camera.view_matrix().inverse(),
            projection: camera.perspective_matrix(),
            projection_inverse: camera.perspective_matrix().inverse(),
            model_view_projection: vp,
            frame,
        }
    }
}

pub struct PerFrameData {
    pub ubo: bolt::Buffer,
    pub desc_set: bolt::DescriptorSet,
}
pub struct AppData {
    pub scene: scene::Scene,
    pub pipeline: ray::Pipeline,
    pub layout_scene: bolt::DescriptorSetLayout,
    pub layout_pass: bolt::DescriptorSetLayout,
    pub pipeline_layout: bolt::PipelineLayout,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,
    pub image_target: bolt::Image2d,
    pub sbt: ray::ShaderBindingTable,
    pub scene_description: ray::SceneDescription,
}

fn create_image_target(context: &Arc<bolt::Context>, window: &bolt::Window) -> bolt::Image2d {
    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(window.get_extent_3d())
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    bolt::Image2d::new(
        context.shared().clone(),
        &image_info,
        vk::ImageAspectFlags::COLOR,
        1,
        "TargetRT"
    )
}

pub fn setup(app: &mut bolt::App) -> AppData {
    let context = &app.renderer.context;
    let index = std::env::args().position(|arg| arg == "--model").unwrap();
    let mut scene = scene::load_scene(
        context.clone(),
        &bolt::util::find_asset(&std::env::args().nth(index + 1).expect("no file given")).unwrap(),
    );
    let mut camera = scene::Camera::new(app.window.get_size());
    camera.look_at(Vec3::splat(5.0), Vec3::ZERO, -Vec3::Y);

    let mut per_frame = Vec::<PerFrameData>::new();

    let mut layout_scene = bolt::DescriptorSetLayout::new(
        context.clone(),
        bolt::DescriptorSetLayoutInfo::default().binding(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::ALL,
        ),
    );
    let layout_pass = bolt::DescriptorSetLayout::new(
        context.clone(),
        bolt::DescriptorSetLayoutInfo::default()
            .binding(
                0,
                vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .binding(
                1,
                vk::DescriptorType::STORAGE_IMAGE,
                vk::ShaderStageFlags::RAYGEN_KHR,
            ),
    );

    let pipeline_layout = bolt::PipelineLayout::new(
        context.clone(),
        bolt::PipelineLayoutInfo::default()
            .desc_set_layouts(&[layout_scene.handle(), layout_pass.handle()]),
    );

    let pipeline = ray::Pipeline::new(
        context.clone(),
        ray::PipelineInfo::default()
            .layout(pipeline_layout.handle())
            .shader(
                bolt::util::find_asset("glsl/debug.rgen").unwrap(),
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .shader(
                bolt::util::find_asset("glsl/debug.rmiss").unwrap(),
                vk::ShaderStageFlags::MISS_KHR,
            )
            .shader(
                bolt::util::find_asset("glsl/debug.rchit").unwrap(),
                vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            )
            .name("debug_mat".to_string()),
    );

    for _ in 0..app.renderer.get_frames_count() {
        let uniforms = SceneUniforms::from(
            &camera,
            uvec3(
                app.window.get_width(),
                app.window.get_height(),
                0,
            ),
        );
        let ubo = bolt::Buffer::from_data(
            context.clone(),
            bolt::BufferInfo::default()
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .cpu_to_gpu(),
            &[uniforms],
        );
        let desc_set = layout_scene
            .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
        per_frame.push(PerFrameData { ubo, desc_set });
    }

    let scene_description = ray::SceneDescription::from_scene(context.clone(), &mut scene);

    let sbt = ray::ShaderBindingTable::new(
        context.clone(),
        pipeline.handle(),
        ray::ShaderBindingTableInfo::default()
            .raygen(0)
            .miss(1)
            .hitgroup(2),
    );

    let image_target = create_image_target(&context, &app.window);

    AppData {
        scene,
        pipeline,
        layout_scene,
        layout_pass,
        pipeline_layout,
        per_frame,
        manip: scene::CameraManip {
            camera,
            input: scene::CameraInput::default(),
            ..Default::default()
        },
        image_target,
        sbt,
        scene_description,
    }
}

pub fn window_event(app: &mut bolt::App, data: &mut AppData, event: &WindowEvent) {
    data.manip.update(&event);
    match event {
        WindowEvent::Resized(_) => {
            data.image_target = create_image_target(&app.renderer.context, &app.window);
            data.layout_pass.reset_pool();
        }
        _ => {}
    }
}

pub fn render(app: &mut bolt::App, data: &mut AppData) -> Result<(), bolt::AppRenderError> {
    let (semaphore, frame_index) = app.renderer.acquire_next_image()?;

    let ref mut frame_ubo = data.per_frame[frame_index].ubo;
    frame_ubo.update(&[SceneUniforms::from(
        &data.manip.camera,
        uvec3(
            app.window.get_width(),
            app.window.get_height(),
            app.elapsed_ticks as u32,
        ),
    )]);

    let cmd = app.renderer.begin_command_buffer();

    let desc_scene = data.per_frame[frame_index].desc_set.handle();

    data.image_target.transition_image_layout(
        cmd,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::GENERAL,
    );

    let image_info = vk::DescriptorImageInfo::builder()
        .image_view(data.image_target.get_image_view())
        .image_layout(vk::ImageLayout::GENERAL)
        .build();
    let desc_pass = data.layout_pass.get_or_create(
        bolt::DescriptorSetInfo::default()
            .accel_struct(0, data.scene_description.tlas().handle())
            .image(1, image_info),
    );

    let device = app.renderer.context.device();
    unsafe {
        device.cmd_set_scissor(cmd, 0, &[app.window.get_rect()]);
        device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport()]);
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            data.pipeline.handle(),
        );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            data.pipeline_layout.handle(),
            0,
            &[desc_scene, desc_pass.handle()],
            &[],
        );
    }
    data.sbt.cmd_trace_rays(cmd, app.window.get_extent_3d());

    let present_image = app.renderer.swapchain.get_present_image(frame_index);
    data.image_target.cmd_blit_to(cmd, present_image, true);
    present_image.transition_image_layout(
        cmd,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::PRESENT_SRC_KHR,
    );
    app.renderer.end_command_buffer(cmd);
    app.renderer.submit_and_present(cmd, semaphore)
}

pub fn prepare() -> bolt::AppSettings {
    bolt::AppSettings {
        name: "Raytracing App".to_string(),
        resolution: [900, 600],
        render: bolt::RendererSettings {
            extensions: vec![vk::KhrGetPhysicalDeviceProperties2Fn::name()],
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn main() {
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}