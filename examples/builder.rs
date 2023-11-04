use std::env;
//#![windows_subsystem = "windows"]
use bolt::prelude::*;
use bolt::scene;
use rayon::prelude::*;

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
    pub graphics_pipeline: bolt::Pipeline,
    // pub compute_pipeline: bolt::Pipeline,
    pub desc_set_layout: bolt::DescriptorSetLayout,
    pub pipeline_layout: bolt::PipelineLayout,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,
}

pub struct AppDataBuilder<'a> {
    pub app: &'a bolt::App,

    // all fields of AppData but Optional
    pub scene: Option<scene::Scene>,
    pub graphics_pipeline: Option<bolt::Pipeline>,
    pub compute_pipeline: Option<bolt::Pipeline>,
    pub desc_set_layout: Option<bolt::DescriptorSetLayout>,
    pub pipeline_layout: Option<bolt::PipelineLayout>,
    pub per_frame: Option<Vec<PerFrameData>>,
    pub manip: Option<scene::CameraManip>,
}

impl<'b, 'a> AppDataBuilder<'a> {
    pub fn new(app: &'b mut bolt::App) -> AppDataBuilder {
        AppDataBuilder {
            app: app,
            scene: None,
            graphics_pipeline: None,
            compute_pipeline: None,
            desc_set_layout: None,
            pipeline_layout: None,
            per_frame: None,
            manip: None,
        }
    }
    pub fn scene(mut self, scene: scene::Scene) -> Self {
        self.scene = Some(scene);
        self
    }
    pub fn descriptor_set_layout(mut self, info: bolt::DescriptorSetLayoutInfo) -> Self {
        self.desc_set_layout = Some(bolt::DescriptorSetLayout::new(
            self.app.renderer.context.clone(),
            info
        ));
        self
    }

    fn pipeline_layout(mut self) -> Self {
        self.pipeline_layout = Some(bolt::PipelineLayout::new(
            self.app.renderer.context.clone(),
            bolt::PipelineLayoutInfo::default().desc_set_layout(self.desc_set_layout.as_ref().expect("specify a descriptor set layout before building the pipeline layout").handle()),
        ));
        self
    }

    fn graphics_pipeline<F: FnOnce(bolt::PipelineInfo) -> bolt::PipelineInfo>(mut self, pipeline_opts: F) -> Self {
        self.graphics_pipeline = Some(bolt::Pipeline::new(
            self.app.renderer.context.clone(),
            pipeline_opts(
                bolt::PipelineInfo::default()
                .layout(self.pipeline_layout.as_ref().expect("specify a pipeline layout before building the pipeline").handle())
                .render_pass_info(self.app.renderer.swapchain.get_transient_render_pass_info())
            )
        ));
        self
    }

    fn per_frame<F: Fn(&mut AppDataBuilder) -> PerFrameData>(mut self, data: F) -> Self {
        self.per_frame = Some((0..self.app.renderer.get_frames_count()).into_iter().map(|_| data(& mut self)).collect());   
        self
    }

    fn manip(mut self, manip: scene::CameraManip) -> Self {
        self.manip = Some(manip);
        self
    }

    fn build(self) -> AppData {
        AppData {
            scene: self.scene.expect("specify a scene before building the app data"),
            graphics_pipeline: self.graphics_pipeline.expect("specify a graphics pipeline before building the app data"),
            // compute_pipeline: self.compute_pipeline.expect("specify a compute pipeline before building the app data"),
            desc_set_layout: self.desc_set_layout.expect("specify a descriptor set layout before building the app data"),
            pipeline_layout: self.pipeline_layout.expect("specify a pipeline layout before building the app data"),
            per_frame: self.per_frame.expect("specify per frame data before building the app data"),
            manip: self.manip.expect("specify a camera manipulator before building the app data"),
        }
    }


}


pub fn setup(app: &mut bolt::App) -> AppData {
    let context = app.renderer.context.clone();
    let index = std::env::args().position(|arg| arg == "--model").unwrap();
    let scene = scene::load_scene(
        context.clone(),
        &bolt::util::find_asset(&std::env::args().nth(index + 1).expect("no file given"))
            .unwrap(),
    );
    let window_size = app.window.get_size();
    let mut camera = scene::Camera::new(window_size);
    camera.look_at(Vec3::splat(3.0), vec3(0.0, 0.5, 0.0), -Vec3::Y);

    let scene_data = SceneData {
        mvp: camera.perspective_matrix() * camera.view_matrix() * scene.meshes[0].transform,
        normal: (camera.view_matrix() * scene.meshes[0].transform)
            .inverse()
            .transpose(),
    };

    let per_frame = |b: &mut AppDataBuilder| -> PerFrameData {
        let ubo = bolt::Buffer::from_data(
            context.clone(),
            bolt::BufferInfo::default()
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .cpu_to_gpu(),
            &[scene_data],
        );
        let desc_set = b.desc_set_layout.as_mut().expect("descriptor set layout not specified yet.")
            .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
        PerFrameData { ubo, desc_set }
    };

    AppDataBuilder::new(app)
    .scene(scene)
    .descriptor_set_layout(
        bolt::DescriptorSetLayoutInfo::default().binding(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::ALL,
        )
    )
    .pipeline_layout()
    .graphics_pipeline(|pipeline_info| {
        pipeline_info
            .vert(bolt::util::find_asset("glsl/model.vert").unwrap())
            .frag(bolt::util::find_asset("glsl/model.frag").unwrap())
            .front_face(vk::FrontFace::CLOCKWISE)
            .vertex_type::<scene::ModelVertex>()
    })
    .per_frame(per_frame)
    .manip(scene::CameraManip {
        camera,
        input: scene::CameraInput::default(),
        mode: scene::CameraMode::Fly,
        ..Default::default()
    })
    .build()
}

pub fn window_event(_: &mut bolt::App, data: &mut AppData, event: &winit::event::WindowEvent) {
    data.manip.update(&event);
}

pub fn render(app: &mut bolt::App, data: &mut AppData) -> Result<(), bolt::AppRenderError> {
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
        device.cmd_set_scissor(cmd, 0, &[app.window.get_rect()]);
        device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport()]);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, data.graphics_pipeline.handle());
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            descriptor_sets.as_slice(),
            &[],
        );
    }
    data.scene.vulkan_meshes.par_iter().for_each(|mesh| mesh.cmd_draw(cmd));
    app.renderer.end_frame_default(image_aquired_semaphore, cmd)
}

pub fn prepare() -> bolt::AppSettings {
    bolt::AppSettings {
        name: "SPH".to_string(),
        resolution: [1500, 800],
        render: bolt::RendererSettings {
            samples: 8,
            clear_color: Vec4::splat(0.15),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
