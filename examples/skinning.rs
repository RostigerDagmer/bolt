use std::env;

//#![windows_subsystem = "windows"]
use bolt::prelude::*;
use bolt::scene;
use bolt::scene::CameraManip;
use bolt::scene::Scene;
use bolt::util::BasicVertex;
use bolt::util::colored_cube_vertices;
use bolt::util::cube_vertices;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct SceneData {
    mvp: Mat4,
    normal: Mat4,
    model: Mat4,
    view: Mat4,
    projection: Mat4,
    model_view: Mat4,
    view_projection: Mat4,
}

pub struct PerFrameData {
    pub ubo: bolt::Buffer,
    pub desc_set: bolt::DescriptorSet,
}

pub struct AppData {
    pub scene: scene::Scene,
    pub joint_geometry: bolt::Buffer,
    pub desc_set_layout: bolt::DescriptorSetLayout,
    pub pipeline_layout: bolt::PipelineLayout,
    pub pass_layout: bolt::DescriptorSetLayout,
    pub graphics_pipeline: bolt::Pipeline,
    pub debug_pipeline: bolt::Pipeline,
    pub compute_pipeline: bolt::ComputePipeline,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,
}

pub struct AppDataBuilder<'a> {
    pub app: &'a bolt::App,

    // all fields of AppData but Optional
    pub scene: Option<Scene>,
    pub graphics_pipeline: Option<bolt::Pipeline>,
    pub compute_pipeline: Option<bolt::ComputePipeline>,
    pub debug_pipeline: Option<bolt::Pipeline>,
    pub desc_set_layout: Option<bolt::DescriptorSetLayout>,
    pub pass_layout: Option<bolt::DescriptorSetLayout>,
    pub pipeline_layout: Option<bolt::PipelineLayout>,
    pub per_frame: Option<Vec<PerFrameData>>,
    pub manip: Option<scene::CameraManip>,
}

impl<'b: 'a, 'a> AppDataBuilder<'a> {
    pub fn new(app: &'b mut bolt::App) -> AppDataBuilder {
        AppDataBuilder {
            app: app,
            scene: None,
            graphics_pipeline: None,
            compute_pipeline: None,
            debug_pipeline: None,
            desc_set_layout: None,
            pass_layout: None,
            pipeline_layout: None,
            per_frame: None,
            manip: None,
        }
    }
    pub fn scene(mut self, scene: Scene) -> Self {
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

    fn pipeline_layout<F: FnOnce(bolt::PipelineLayoutInfo) -> bolt::PipelineLayoutInfo>(mut self, extensions: Option<F>) -> Self {

        let layout = bolt::PipelineLayoutInfo::default().desc_set_layouts(&[
            self.desc_set_layout.as_ref().expect("specify a descriptor set layout before building the pipeline layout").handle(),
            self.pass_layout.as_ref().expect("specify a descriptor set layout for the render pass before building the pipeline layout").handle(),
            ]);
        let layout =  match extensions {
            Some(f) => f(layout),
            None => layout,
        };
        self.pipeline_layout = Some(bolt::PipelineLayout::new(
            self.app.renderer.context.clone(),
            layout,
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

    fn compute_pipeline<F: FnOnce(bolt::ComputePipelineInfo) -> bolt::ComputePipelineInfo>(mut self, pipeline_opts: F) -> Self {
        self.compute_pipeline = Some(bolt::ComputePipeline::new(
            self.app.renderer.context.clone(),
            pipeline_opts(
                bolt::ComputePipelineInfo::default()
                .layout(self.pipeline_layout.as_ref().expect("specify a pipeline layout before building the pipeline").handle())
            )
        ));
        self
    }

    fn debug_pipeline<F: FnOnce(bolt::PipelineInfo) -> bolt::PipelineInfo>(mut self, pipeline_opts: F) -> Self {
        self.debug_pipeline = Some(bolt::Pipeline::new(
            self.app.renderer.context.clone(),
            pipeline_opts(
                bolt::PipelineInfo::default()
                .layout(self.pipeline_layout.as_ref().expect("specify a pipeline layout before building the pipeline").handle())
                .render_pass_info(self.app.renderer.swapchain.get_transient_render_pass_info())
            )
        ));
        self
    }

    fn pass_layout(mut self, info: bolt::DescriptorSetLayoutInfo) -> Self {
        self.pass_layout = Some(bolt::DescriptorSetLayout::new(
            self.app.renderer.context.clone(),
            info
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

        let joint_geometry = bolt::Buffer::from_data(
            self.app.renderer.context.clone(),
            bolt::BufferInfo::default()
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .gpu_only(),
            &cube_vertices(0.8),
        );

        AppData {
            scene: self.scene.expect("specify a scene before building the app data"),
            joint_geometry,
            graphics_pipeline: self.graphics_pipeline.expect("specify a graphics pipeline before building the app data"),
            compute_pipeline: self.compute_pipeline.expect("specify a compute pipeline before building the app data"),
            debug_pipeline: self.debug_pipeline.expect("specify a debug pipeline before building the app data"),
            desc_set_layout: self.desc_set_layout.expect("specify a descriptor set layout before building the app data"),
            pass_layout: self.pass_layout.expect("specify a descriptor set layout for the render pass before building the app data"),
            pipeline_layout: self.pipeline_layout.expect("specify a pipeline layout before building the app data"),
            per_frame: self.per_frame.expect("specify per frame data before building the app data"),
            manip: self.manip.expect("specify a camera manipulator before building the app data"),
        }
    }
}

struct JointTransform {
    pub transform: Mat4,
}

impl Vertex for JointTransform {
    fn stride() -> u32 {
        std::mem::size_of::<JointTransform>() as u32
    }
    fn format_offset() -> Vec<(ash::vk::Format, u32)> {
        vec![
            (vk::Format::R32G32B32A32_SFLOAT, 0),
            (vk::Format::R32G32B32A32_SFLOAT, 16),
            (vk::Format::R32G32B32A32_SFLOAT, 32),
            (vk::Format::R32G32B32A32_SFLOAT, 48)   
        ]
    }
}

pub fn setup(app: &mut bolt::App) -> AppData {
    let index = std::env::args().position(|arg| arg == "--model").unwrap();
    let scene = scene::load_scene(
        app.renderer.context.clone(),
        &bolt::util::find_asset(&std::env::args().nth(index + 1).expect("no file given"))
            .unwrap(),
    );
    let mut camera = scene::Camera::new( app.window.get_size());
    camera.look_at(vec3(150.0, 125.0, 250.0), vec3(0.0, 100.0, 0.0), -Vec3::Y);

    let scene_data = SceneData {
        mvp: camera.perspective_matrix() * camera.view_matrix() * scene.meshes[0].transform,
        normal: (camera.view_matrix() * scene.meshes[0].transform)
            .inverse()
            .transpose(),
        model: scene.meshes[0].transform,
        view: camera.view_matrix(),
        projection: camera.perspective_matrix(),
        model_view: camera.view_matrix() * scene.meshes[0].transform,   
        view_projection: camera.perspective_matrix() * camera.view_matrix(),    
    };

    AppDataBuilder::new(app)
        .scene(scene)
        .descriptor_set_layout(bolt::DescriptorSetLayoutInfo::default().binding(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::ALL,
        ))
        .pass_layout(bolt::DescriptorSetLayoutInfo::default()
            .binding(0,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
            )
            .binding(1,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
            )
            .binding(2, vk::DescriptorType::STORAGE_BUFFER, vk::ShaderStageFlags::COMPUTE) // transforms
            .binding(3, vk::DescriptorType::STORAGE_BUFFER, vk::ShaderStageFlags::COMPUTE) // joints
            .binding(4, vk::DescriptorType::STORAGE_BUFFER, vk::ShaderStageFlags::COMPUTE) // inverse bind matrices
            .binding(5, vk::DescriptorType::STORAGE_BUFFER, vk::ShaderStageFlags::ALL_GRAPHICS)  
        )
        .pipeline_layout(Some(|layout_info| { layout_info }))
        .graphics_pipeline(|layout| {
            layout
            .vert(bolt::util::find_asset("glsl/model.vert").unwrap())
            .frag(bolt::util::find_asset("glsl/model.frag").unwrap())
            .front_face(vk::FrontFace::CLOCKWISE)
            .cull_mode(vk::CullModeFlags::BACK)
            .vertex_type::<scene::ModelVertex>()
            .polygon_mode(vk::PolygonMode::FILL)
        })
        .compute_pipeline(|layout| {
            layout
                .comp(bolt::util::find_asset("glsl/skinning.comp").unwrap())
        })
        .debug_pipeline(|layout| {
            layout
            .vert(bolt::util::find_asset("glsl/debug/debug.vert").unwrap())
            .frag(bolt::util::find_asset("glsl/debug/debug.frag").unwrap())
            // .vert(bolt::util::find_asset("glsl/model.vert").unwrap())
            // .frag(bolt::util::find_asset("glsl/model.frag").unwrap())
            .front_face(vk::FrontFace::CLOCKWISE)
            .vertex_type::<BasicVertex>()
            .instance_type::<JointTransform>()
            .polygon_mode(vk::PolygonMode::LINE)
            .cull_mode(vk::CullModeFlags::NONE)
            // .vertex_stride(std::mem::size_of::<BasicVertex>() as u32)
        })
        .per_frame(|builder| {
            let ubo = bolt::Buffer::from_data(
                builder.app.renderer.context.clone(),
                bolt::BufferInfo::default()
                    .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                    .cpu_to_gpu(),
                &[scene_data],
            );
            let desc_set = builder
                .desc_set_layout
                .as_mut()
                .unwrap()
                .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
            PerFrameData { ubo, desc_set }
        })
        .manip(CameraManip {
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

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    pub weights_count: u32,
    pub vertex_count: u32,
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
        model: data.scene.meshes[0].transform,
        view: camera.view_matrix(),
        projection: camera.perspective_matrix(),
        model_view: camera.view_matrix() * data.scene.meshes[0].transform,
        view_projection: camera.perspective_matrix() * camera.view_matrix(),
    };
    data.per_frame[app.renderer.active_frame_index]
        .ubo
        .update(&[scene_data]);
    let pipeline_layout = data.pipeline_layout.handle();

    // greate opportunity for parallelization on multiple skinned meshes
    data.scene.skins.iter_mut()
    .zip(data.scene.vulkan_skins.iter())
    .zip(data.scene.rigs.iter())
    .for_each(|((skin, vk_skin), rig)| {
        skin.transforms_from(rig);
        vk_skin.update(skin);
    });

    let pass_layout = data.pass_layout.get_or_create(
        bolt::DescriptorSetInfo::default()
            .buffer(0, data.scene.vulkan_meshes[0].vertex_buffer.get_descriptor_info())
            // .buffer(1, data.scene.vulkan_meshes[0].index_buffer.unwrap().get_descriptor_info())
            .buffer(2, data.scene.vulkan_skins[0].transforms.get_descriptor_info())
            .buffer(3, data.scene.vulkan_skins[0].joints.get_descriptor_info())
            .buffer(4, data.scene.vulkan_skins[0].inverse_bind_matrices.get_descriptor_info())
            .buffer(5, data.joint_geometry.get_descriptor_info())
    );

    let vertex_count = data.scene.vulkan_meshes[0].vertex_buffer.get_element_count() as u32;
    let weights_count = data.scene.vulkan_skins[0].joints.get_element_count() as u32;
    let joint_count = data.scene.vulkan_skins[0].transforms.get_element_count() as u32;

    let constants = Constants {
        weights_count,
        vertex_count,
    };

    let descriptor_sets = [data.per_frame[app.renderer.active_frame_index].desc_set.handle(), pass_layout.handle()];
    let device = app.renderer.context.device();


    unsafe {
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            data.compute_pipeline.handles().unwrap()[0],
        );
        device.cmd_push_constants(
            cmd,
            data.pipeline_layout.handle(),
            vk::ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::bytes_of(&constants),
        );
        device.cmd_dispatch(cmd, weights_count, 1, 1);
    }

    unsafe {
        // memory barrier
        let buffer_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(data.scene.vulkan_meshes[0].vertex_buffer.handle())
            .offset(0)
            .size(vk::WHOLE_SIZE)
            .build();

        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::VERTEX_SHADER,
            vk::DependencyFlags::default(),
            &[],
            &[buffer_barrier],
            &[],
        );
    }


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

    // insanely bad (maybe scene.draw with all buffers as slices in one go)
    data.scene.vulkan_meshes.iter().for_each(|mesh| mesh.cmd_draw(cmd));

    unsafe {
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, data.debug_pipeline.handle());
        device.cmd_bind_vertex_buffers(
            cmd, 
            0, 
            &[
                data.joint_geometry.handle(),
                data.scene.vulkan_skins[0].global_bone_transforms.handle(),
                ], 
                &[0, 0]
            );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            descriptor_sets.as_slice(),
            &[],
        );
        device.cmd_draw(
            cmd, 
            data.joint_geometry.get_element_count(),
            data.scene.vulkan_skins[0].global_bone_transforms.get_element_count(),
            0,
            0,
        );
    }

    app.renderer.end_frame_default(image_aquired_semaphore, cmd)
}

pub fn prepare() -> bolt::AppSettings {
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
}

pub fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
