#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use super::*;

pub type Shader = &'static [u32];

#[macro_export]
macro_rules! vert_shader
{
    ($path:expr) =>
    {
        if gru_vulkan::DEBUG_MODE { include_spirv!($path, glsl, vert) }
        else { include_spirv!($path, glsl, vert, max_perf, no_debug) }
    }
}

#[macro_export]
macro_rules! frag_shader
{
    ($path:expr) =>
    {
        if gru_vulkan::DEBUG_MODE { include_spirv!($path, glsl, frag) }
        else { include_spirv!($path, glsl, frag, max_perf, no_debug) }
    }
}

impl Device
{
	pub fn new_pipeline_layout(&self, descriptors: &[&DescriptorSetLayout]) -> PipelineLayout
    {
        let descriptor_set_layouts: Vec<_> = descriptors.iter().map(|info| info.0.descriptor_set_layout).collect();
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);
        let pipeline_layout = unsafe { self.0.logical_device.create_pipeline_layout(&pipeline_layout_info, None) }.unwrap();
        PipelineLayout { device: self.0.clone(), layout: pipeline_layout }
    }
    
    pub fn new_pipeline
    (
        &self,
        render_pass: &RenderPass,
        subpass: u32,
        vertex_shader_spirv: Shader,
        fragment_shader_spirv: Shader,
        attributes: &[&AttributeGroupInfo],
        layout: &PipelineLayout,
        info: &PipelineInfo
    ) -> Pipeline
    {
        //shader
        let vertex_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_shader_spirv);
        let vertex_shader_module = unsafe { self.0.logical_device.create_shader_module(&vertex_shader_create_info, None) }.unwrap();
        let fragment_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&fragment_shader_spirv);
        let fragment_shader_module = unsafe { self.0.logical_device.create_shader_module(&fragment_shader_create_info, None) }.unwrap();
        let main_function_name = std::ffi::CString::new("main").unwrap();
        let vertex_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&main_function_name);
        let fragment_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&main_function_name);
        let shader_stages = [vertex_shader_stage.build(), fragment_shader_stage.build()];
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
                    InputRate::VERTEX => vk::VertexInputRate::VERTEX,
                    InputRate::INSTANCE => vk::VertexInputRate::INSTANCE
                }
            });
        }
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);
        //extra stuff
        let viewports = [vk::Viewport
        {
            x: info.viewport_origin.0,
            y: info.viewport_origin.1,
            width: info.viewport_size.0,
            height: info.viewport_size.1,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vk::Rect2D
        {
            offset: vk::Offset2D { x: info.scissor_origin.0, y: info.scissor_origin.1 },
            extent: vk::Extent2D { width: info.scissor_size.0, height: info.scissor_size.1 },
        }];
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(info.topology.vk_primitive_topology());
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(info.cull.vk_cull_mode())
            .line_width(info.line_width)
            .polygon_mode(info.polygon.vk_polygon_mode());
        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(info.samples.vk_sample_count())
            .sample_shading_enable(info.min_sample_shading.is_some())
            .min_sample_shading(*info.min_sample_shading.as_ref().unwrap_or_else(|| &0.0));
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(info.depth_test)
            .depth_write_enable(info.depth_test)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(info.blend)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
            .build()];
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachments);
        //bringing all together
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .viewport_state(&viewport_info)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blend_info)
            .layout(layout.layout)
            .render_pass(render_pass.render_pass)
            .subpass(subpass);
        let pipeline = unsafe { self.0.logical_device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info.build()], None) }.unwrap()[0];
        unsafe
        {
            self.0.logical_device.destroy_shader_module(fragment_shader_module, None);
            self.0.logical_device.destroy_shader_module(vertex_shader_module, None);
        };
        Pipeline { device: self.0.clone(), pipeline }
    }
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
pub struct PipelineInfo
{
    pub viewport_origin: (f32, f32),
    pub viewport_size: (f32, f32),
    pub scissor_origin: (i32, i32),
    pub scissor_size: (u32, u32),
    pub topology: PipelineTopology,
    pub samples: Msaa,
    pub min_sample_shading: Option<f32>,
    pub line_width: f32,
    pub polygon: PipelinePolygon,
    pub cull: PipelineCull,
    pub depth_test: bool,
    pub blend: bool
}

/*
impl PipelineInfo
{
    pub fn new(width: u32, height: u32) -> Self
    {
        Self
        {
            viewport_origin: (0.0, 0.0),
            viewport_size: (width as f32, height as f32),
            scissor_origin: (0, 0),
            scissor_size: (width, height),
            topology: PipelineTopology::TriangleList,
            samples: Msaa::None,
            min_sample_shading: None,
            line_width: 1.0,
            polygon: PipelinePolygon::Fill,
            cull: PipelineCull::Back,
            depth_test: true,
            blend: false
        }
    }
}
*/
