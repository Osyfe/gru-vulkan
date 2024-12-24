#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use super::*;

pub type Shader = &'static [u32];

#[macro_export]
macro_rules! comp_shader
{
    ($path:expr) =>
    {
        if cfg!(debug_assertions) { include_spirv!($path, glsl, comp, vulkan1_0) }
        else { include_spirv!($path, glsl, comp, max_perf, no_debug, vulkan1_0) }
    }
}

#[macro_export]
macro_rules! vert_shader
{
    ($path:expr) =>
    {
        if cfg!(debug_assertions) { include_spirv!($path, glsl, vert, vulkan1_0) }
        else { include_spirv!($path, glsl, vert, max_perf, no_debug, vulkan1_0) }
    }
}

#[macro_export]
macro_rules! frag_shader
{
    ($path:expr) =>
    {
        if cfg!(debug_assertions) { include_spirv!($path, glsl, frag, vulkan1_0) }
        else { include_spirv!($path, glsl, frag, max_perf, no_debug, vulkan1_0) }
    }
}

impl Device
{
	pub fn new_pipeline_layout(&self, descriptors: &[&DescriptorSetLayout], push_constant: Option<PushConstantInfo>) -> PipelineLayout
    {
        let descriptor_set_layouts: Vec<_> = descriptors.iter().map(|info| info.0.descriptor_set_layout).collect();
        let mut push_constant_ranges = Vec::new();
        let push_constant = if let Some(push_constant) = &push_constant
        {
            if DEBUG_MODE
            {
                if push_constant.size % 4 != 0 { panic!("Device::new_pipeline_layout: Push constant size is not a multiple of 4."); }
                if push_constant.size > 128 { panic!("Device::new_pipeline_layout: Push constant size is larger than 128 bytes."); }
            }
            let shader_stages =
                if !push_constant.vertex && !push_constant.fragment { panic!("Device::new_pipeline_layout: Push constant neither in vertex nor fragment shader."); }
                else if push_constant.vertex { vk::ShaderStageFlags::VERTEX }
                else if push_constant.fragment { vk::ShaderStageFlags::FRAGMENT }
                else { vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT };
            push_constant_ranges.push(vk::PushConstantRange { stage_flags: shader_stages, offset: 0, size: push_constant.size });
            Some((shader_stages, push_constant.size))
        } else { None };
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_constant_ranges);
        let pipeline_layout = unsafe { self.0.logical_device.create_pipeline_layout(&pipeline_layout_info, None) }.unwrap();
        PipelineLayout { device: self.0.clone(), layout: pipeline_layout, push_constant }
    }
    
    pub fn new_pipeline
    (
        &self,
        render_pass: &RenderPassLayout,
        subpass: u32,
        vertex_shader_spirv: Shader,
        fragment_shader_spirv: Shader,
        attributes: &[AttributeGroupInfo],
        layout: &PipelineLayout,
        info: &PipelineInfo
    ) -> Pipeline
    {
        //shader
        let vertex_shader_create_info = vk::ShaderModuleCreateInfo::default().code(&vertex_shader_spirv);
        let vertex_shader_module = unsafe { self.0.logical_device.create_shader_module(&vertex_shader_create_info, None) }.unwrap();
        let fragment_shader_create_info = vk::ShaderModuleCreateInfo::default().code(&fragment_shader_spirv);
        let fragment_shader_module = unsafe { self.0.logical_device.create_shader_module(&fragment_shader_create_info, None) }.unwrap();
        let main_function_name = std::ffi::CString::new("main").unwrap();
        let vertex_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&main_function_name);
        let fragment_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&main_function_name);
        let shader_stages = [vertex_shader_stage, fragment_shader_stage];
        //attributes
        let mut vertex_binding_descriptions = Vec::new();
        let mut vertex_attribute_descriptions = Vec::new();
        for (binding, group) in attributes.iter().enumerate()
        {
            let mut offset = 0;
            for attribute in group.attributes
            {
                vertex_attribute_descriptions.push(vk::VertexInputAttributeDescription
                {
                    binding: binding as u32,
                    location: attribute.0.0,
                    offset,
                    format: attribute.1.vk_format()
                });
                offset += attribute.1.size_in_bytes();
            }
            vertex_binding_descriptions.push(vk::VertexInputBindingDescription
            {
                binding: binding as u32,
                stride: offset,
                input_rate: match group.rate
                {
                    InputRate::Vertex => vk::VertexInputRate::VERTEX,
                    InputRate::Instance => vk::VertexInputRate::INSTANCE
                }
            });
        }
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);
        //extra stuff
        let (viewport, scissor) = info.view.as_ref().unwrap_or(&ViewInfo::dummy()).build();
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));
        let dynamic_state =
            if info.view.is_none() { vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]) }
            else { vk::PipelineDynamicStateCreateInfo::default() };
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(info.topology.vk_primitive_topology());
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(info.depth_test.depth_clamp_enable())
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(info.cull.vk_cull_mode())
            .line_width(info.line_width)
            .polygon_mode(info.polygon.vk_polygon_mode());
        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(info.samples.vk_sample_count())
            .sample_shading_enable(info.min_sample_shading.is_some())
            .min_sample_shading(*info.min_sample_shading.as_ref().unwrap_or_else(|| &0.0));
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(info.depth_test.depth_test_enable())
            .depth_write_enable(info.depth_test.depth_test_enable())
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
        let color_blend_attachments =
        [
            vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(info.blend)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
        ];
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&color_blend_attachments);
        //bringing all together + static vs dynamic state
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blend_info)
            .layout(layout.layout)
            .render_pass(render_pass.raw.render_pass)
            .subpass(subpass);
        let pipeline = unsafe { self.0.logical_device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) }.unwrap()[0];
        unsafe
        {
            self.0.logical_device.destroy_shader_module(fragment_shader_module, None);
            self.0.logical_device.destroy_shader_module(vertex_shader_module, None);
        }
        Pipeline { device: self.0.clone(), pipeline }
    }
}

#[derive(Clone, Copy)]
pub struct PushConstantInfo
{
    pub vertex: bool,
    pub fragment: bool,
    pub size: u32
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PipelineTopology
{
    PointList,
    LineList,
    TriangleList
}

impl PipelineTopology
{
    const fn vk_primitive_topology(&self) -> vk::PrimitiveTopology
    {
        match self
        {
            PipelineTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
            PipelineTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
            PipelineTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PipelinePolygon
{
    Point,
    Line,
    Fill
}

impl PipelinePolygon
{
    const fn vk_polygon_mode(&self) -> vk::PolygonMode
    {
        match self
        {
            PipelinePolygon::Point => vk::PolygonMode::POINT,
            PipelinePolygon::Line => vk::PolygonMode::LINE,
            PipelinePolygon::Fill => vk::PolygonMode::FILL
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PipelineCull
{
    None,
    Front,
    Back
}

impl PipelineCull
{
    const fn vk_cull_mode(&self) -> vk::CullModeFlags
    {
        match self
        {
            PipelineCull::None => vk::CullModeFlags::NONE,
            PipelineCull::Front => vk::CullModeFlags::FRONT,
            PipelineCull::Back => vk::CullModeFlags::BACK
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DepthTest
{
    None,
    Normal,
    Clamp
}

impl DepthTest
{
    const fn depth_test_enable(&self) -> bool
    {
        match self
        {
            DepthTest::None => false,
            DepthTest::Normal | DepthTest::Clamp => true
        }
    }
    
    const fn depth_clamp_enable(&self) -> bool
    {
        match self
        {
            DepthTest::None | DepthTest::Normal => false,
            DepthTest::Clamp => true
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PipelineInfo
{
    pub view: Option<ViewInfo>,
    pub topology: PipelineTopology,
    pub samples: Msaa,
    pub min_sample_shading: Option<f32>,
    pub line_width: f32,
    pub polygon: PipelinePolygon,
    pub cull: PipelineCull,
    pub depth_test: DepthTest,
    pub blend: bool
}

impl ViewInfo
{
    pub fn full(width: u32, height: u32) -> Self
    {
        Self
        {
            viewport_origin: (0.0, 0.0),
            viewport_size: (width as f32, height as f32),
            scissor_origin: (0, 0),
            scissor_size: (width, height)
        }
    }

    pub(crate) fn dummy() -> Self
    {
        Self
        {
            viewport_origin: (0.0, 0.0),
            viewport_size: (0.0, 0.0),
            scissor_origin: (0, 0),
            scissor_size: (0, 0)
        }
    }

    pub(crate) fn build(&self) -> (vk::Viewport, vk::Rect2D)
    {
        let viewport = vk::Viewport
        {
            x: self.viewport_origin.0,
            y: self.viewport_origin.1,
            width: self.viewport_size.0,
            height: self.viewport_size.1,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D
        {
            offset: vk::Offset2D { x: self.scissor_origin.0, y: self.scissor_origin.1 },
            extent: vk::Extent2D { width: self.scissor_size.0, height: self.scissor_size.1 },
        };
        (viewport, scissor)
    }
}
