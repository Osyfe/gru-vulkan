use super::*;

impl Device
{
    pub fn new_compute(&self, compute_shader_spirv: Shader, layout: &PipelineLayout) -> Compute
    {
        let compute_shader_create_info = vk::ShaderModuleCreateInfo::default().code(&compute_shader_spirv);
        let compute_shader_module = unsafe { self.0.logical_device.create_shader_module(&compute_shader_create_info, None) }.unwrap();
        let main_function_name = std::ffi::CString::new("main").unwrap();
        let compute_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_shader_module)
            .name(&main_function_name);
        let compute_info = vk::ComputePipelineCreateInfo::default()
            .stage(compute_shader_stage)
            .layout(layout.layout);
        let compute = unsafe { self.0.logical_device.create_compute_pipelines(vk::PipelineCache::null(), &[compute_info], None) }.unwrap()[0];
        unsafe { self.0.logical_device.destroy_shader_module(compute_shader_module, None); }
        Compute { device: self.0.clone(), compute }
    }
}
