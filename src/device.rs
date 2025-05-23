use super::*;

impl QueueFamily
{
    pub fn get_queue(&self, queue_index: usize) -> Arc<Mutex<Queue>>
    {
        self.queues[queue_index].clone()
    }

    pub fn count(&self) -> usize { self.queues.len() }
    pub fn supports_graphics(&self) -> bool { self.flags.contains(vk::QueueFlags::GRAPHICS) }
    pub fn supports_compute(&self) -> bool { self.flags.contains(vk::QueueFlags::COMPUTE) }
    pub fn supports_transfer(&self) -> bool { self.flags.contains(vk::QueueFlags::TRANSFER) }
    //pub fn supports_sparse_binding(&self) -> bool { self.queue_family_properties.queue_flags.contains(vk::QueueFlags::SPARSE_BINDING) }
    pub fn supports_surface(&self) -> bool { self.surface_support }
}

impl Device
{
    pub fn idle(&self)
    {
        unsafe { self.0.logical_device.device_wait_idle() }.unwrap();
    }

    pub fn get_queue_family(&self, queue_family_info: &QueueFamilyInfo) -> &QueueFamily
    {
        &self.0.queue_families.iter().filter(|family| family.index == queue_family_info.index).nth(0).unwrap()
    }

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

    pub fn new_semaphore(&self, wait_stage: WaitStage) -> Semaphore
    {
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let semaphore = unsafe { self.0.logical_device.create_semaphore(&semaphore_create_info, None) }.unwrap();
        Semaphore { device: self.0.clone(), semaphore, wait_stage: wait_stage.vk_mask() }
    }

    pub fn new_fence(&self, signaled: bool) -> Fence
    {
        let mut fence_create_info = vk::FenceCreateInfo::default();
        if signaled { fence_create_info = fence_create_info.flags(vk::FenceCreateFlags::SIGNALED) };
        let fence = unsafe { self.0.logical_device.create_fence(&fence_create_info, None) }.unwrap();
        Fence { device: self.0.clone(), fence }
    }
}

impl WaitStage
{
    pub(crate) fn vk_mask(self) -> vk::PipelineStageFlags
    {
        match self
        {
            Self::None => vk::PipelineStageFlags::empty(),
            Self::VertexInput => vk::PipelineStageFlags::VERTEX_INPUT,
            Self::ColorOutput => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            Self::Compute => vk::PipelineStageFlags::COMPUTE_SHADER
        }
    }
}

impl Semaphore
{
    pub fn signal(&self, queue: &Queue)
    {
        let submit_info = vk::SubmitInfo::default()
            .signal_semaphores(std::slice::from_ref(&self.semaphore));
        unsafe { self.device.logical_device.queue_submit(queue.queue, std::slice::from_ref(&submit_info), vk::Fence::null()) }.unwrap();
    }
}

impl Fence
{
    pub fn status(&self) -> bool
    {
        unsafe { self.device.logical_device.get_fence_status(self.fence) }.unwrap()
    }

    pub fn wait(&self)
    {
        unsafe { self.device.logical_device.wait_for_fences(std::slice::from_ref(&self.fence), true, std::u64::MAX) }.unwrap();
    }

    pub fn reset(&self)
    {
        unsafe { self.device.logical_device.reset_fences(std::slice::from_ref(&self.fence)) }.unwrap();
    }
}
