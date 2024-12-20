use super::*;

impl Device
{
    pub fn new_command_pool(&self, queue_family: &QueueFamily) -> CommandPool
    {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family.index as u32)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { self.0.logical_device.create_command_pool(&command_pool_info, None) }.unwrap();
        let raw_pool = RawCommandPool { device: self.0.clone(), pool: command_pool, queue_family_index: queue_family.index, queue_family_flags: queue_family.flags };
        CommandPool { pool: Rc::new(raw_pool) }
    }
}

impl CommandPool
{
    pub fn new_command_buffer(&self) -> CommandBuffer
    {
        let command_bufffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool.pool)
            .command_buffer_count(1);
        let command_buffer = unsafe { self.pool.device.logical_device.allocate_command_buffers(&command_bufffer_allocate_info) }.unwrap()[0];
        CommandBuffer { pool: Rc::clone(&self.pool), command_buffer }
    }
}

#[derive(Clone, Copy)]
pub enum DrawMode
{
    Index { index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32 },
    Vertex { vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32 }
}

impl DrawMode
{
    pub fn index(index_count: u32) -> Self
    {
        Self::Index { index_count, instance_count: 1, first_index: 0, vertex_offset: 0, first_instance: 0 }
    }

    pub fn index_instanced(index_count: u32, instance_count: u32) -> Self
    {
        Self::Index { index_count, instance_count, first_index: 0, vertex_offset: 0, first_instance: 0 }
    }

    pub fn vertex(vertex_count: u32) -> Self
    {
        Self::Vertex { vertex_count, instance_count: 1, first_vertex: 0, first_instance: 0 }
    }

    pub fn vertex_instanced(vertex_count: u32, instance_count: u32) -> Self
    {
        Self::Vertex { vertex_count, instance_count, first_vertex: 0, first_instance: 0 }
    }
}

impl CommandBuffer
{
    #[inline]
    pub fn record<'a>(&'a mut self) -> CommandBufferRecord<'a>
    {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info) }.unwrap();
        CommandBufferRecord { buffer: self }
    }

    #[inline]
    pub fn submit<const N: usize, const M: usize>(&self, queue: &Queue, wait: [&Semaphore; N], signal: [&Semaphore; M], mark: Option<&Fence>)
    {
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::submit: Wrong queue family."); }
        let mut submit_info = vk::SubmitInfo::default()
            .command_buffers(std::slice::from_ref(&self.command_buffer));
        let wait_semaphores = wait.map(|wait| wait.semaphore);
        let wait_dst_stage_mask = wait.map(|wait| wait.wait_stage);
        if N > 0
        {
            submit_info = submit_info
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_dst_stage_mask);
        }
        let signal = signal.map(|signal| signal.semaphore);
        if M > 0
        {
            submit_info = submit_info.signal_semaphores(&signal);
        }
        let submit_info = [submit_info];
        let fence = mark.map(|mark| mark.fence).unwrap_or(vk::Fence::null());
        unsafe { self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, fence) }.unwrap();
    }
}

pub struct CommandBufferRecord<'a>
{
    pub(crate) buffer: &'a mut CommandBuffer
}
            
impl<'a> CommandBufferRecord<'a>
{
    #[inline]
    pub fn bind_compute(&mut self, compute: &Compute) -> &mut Self
    {
        unsafe { self.buffer.pool.device.logical_device.cmd_bind_pipeline(self.buffer.command_buffer, vk::PipelineBindPoint::COMPUTE, compute.compute); }
        self
    }

    #[inline]
    fn bind_descriptor_sets_internal(&mut self, pipeline_layout: &PipelineLayout, descriptor_sets: &[&DescriptorSet], bind_point: vk::PipelineBindPoint) -> &mut Self
    {
        for set in descriptor_sets
        {
            unsafe
            {
                self.buffer.pool.device.logical_device.cmd_bind_descriptor_sets
                (
                    self.buffer.command_buffer,
                    bind_point,
                    pipeline_layout.layout,
                    set.layout.set,
                    &[set.descriptor_set],
                    &[]
                );
            }
        }
        self
    }

    #[inline]
    pub fn bind_descriptor_sets(&mut self, pipeline_layout: &PipelineLayout, descriptor_sets: &[&DescriptorSet]) -> &mut Self
    {
        self.bind_descriptor_sets_internal(pipeline_layout, descriptor_sets, vk::PipelineBindPoint::COMPUTE)
    }

    #[inline]
    pub fn push_constant<T>(&mut self, pipeline_layout: &PipelineLayout, push_constant: &T) -> &mut Self
    {
        let (shader_stages, size) = pipeline_layout.push_constant.expect("CommandBufferRecord::push_constant: This layout has no push constant.");
        if DEBUG_MODE && std::mem::size_of::<T>() as u32 != size { panic!("CommandBufferRecord::push_constant: Incompatible data size."); }
        unsafe
        {
            let data = std::slice::from_raw_parts(push_constant as *const T as *const u8, std::mem::size_of::<T>());
            self.buffer.pool.device.logical_device.cmd_push_constants(self.buffer.command_buffer, pipeline_layout.layout, shader_stages, 0, data);
        }
        self
    }

    #[inline]
    pub fn dispatch(&mut self, group_counts: [u32; 3]) -> &mut Self
    {
        unsafe { self.buffer.pool.device.logical_device.cmd_dispatch(self.buffer.command_buffer, group_counts[0], group_counts[1], group_counts[2]); }
        self
    }

    #[inline]
    pub fn render_pass<'b>(&'b mut self, render_pass: &RenderPass, framebuffer: &Framebuffer) -> CommandBufferRecordRenderPass<'a, 'b>
    {
        let (width, height) = framebuffer.size;
        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass.render_pass)
            .framebuffer(framebuffer.framebuffer)
            .render_area(vk::Rect2D
            {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width, height },
            })
            .clear_values(&render_pass.clear_values);
        unsafe { self.buffer.pool.device.logical_device.cmd_begin_render_pass(self.buffer.command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE); }
        CommandBufferRecordRenderPass { record: self }
    }
/*
    pub fn pipeline_barrier(&mut self, image: &Image) -> &mut Self
    {
        if DEBUG_MODE { if let ImageUsage::Attachment { texture: false, .. } = image.image_usage { panic!("CommandBufferRecord::pipeline_barrier: This attachment cannot be sampled."); } }
        let depth = image.image_usage.depth();
        let barrier = vk::ImageMemoryBarrier::default()
            .image(image.image)
            .src_access_mask(if depth { vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE } else { vk::AccessFlags::COLOR_ATTACHMENT_WRITE })
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(ImageLayout::Attachment.vk_image_layout(depth))
            .new_layout(ImageLayout::Shader.vk_image_layout(depth))
            .subresource_range(vk::ImageSubresourceRange
            {
                aspect_mask: if depth { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::COLOR },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .build();
        unsafe
        {
            self.buffer.device.logical_device.cmd_pipeline_barrier
            (
                self.buffer.command_buffer,
                if depth { vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS } else { vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT },
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[], &[], &[barrier]
            );
        }
        self
    }
    */
}

impl Drop for CommandBufferRecord<'_>
{
    #[inline]
    fn drop(&mut self)
    {
        unsafe { self.buffer.pool.device.logical_device.end_command_buffer(self.buffer.command_buffer) }.unwrap();
    }
}

pub struct CommandBufferRecordRenderPass<'a, 'b>
{
    pub(crate) record: &'b mut CommandBufferRecord<'a>
}

impl<'a, 'b> CommandBufferRecordRenderPass<'a, 'b>
{
    #[inline]
    pub fn next_subpass(&mut self) -> &mut Self
    {
        unsafe { self.record.buffer.pool.device.logical_device.cmd_next_subpass(self.record.buffer.command_buffer, vk::SubpassContents::INLINE); }
        self
    }

    #[inline]
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline) -> &mut Self
    {
        unsafe { self.record.buffer.pool.device.logical_device.cmd_bind_pipeline(self.record.buffer.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline); }
        self
    }

    #[inline]
    pub fn set_view(&mut self, view_info: ViewInfo) -> &mut Self
    {
        let (viewport, scissor) = view_info.build();
        unsafe
        {
            self.record.buffer.pool.device.logical_device.cmd_set_viewport(self.record.buffer.command_buffer, 0, &[viewport]);
            self.record.buffer.pool.device.logical_device.cmd_set_scissor(self.record.buffer.command_buffer, 0, &[scissor]);
        }
        self
    }

    #[inline]
    pub fn bind_indices(&mut self, indices: IndexBinding) -> &mut Self
    {
        unsafe { self.record.buffer.pool.device.logical_device.cmd_bind_index_buffer(self.record.buffer.command_buffer, indices.buffer.buffer, indices.offset_in_bytes, indices.format); }
        self
    }

    #[inline]
    pub fn bind_attributes<const N: usize>(&mut self, first_binding: u32, attributes: [AttributeBinding; N]) -> &mut Self
    {
        let (mut buffers, mut offsets_in_bytes) = ([Default::default(); N], [Default::default(); N]);
        for (i, binding) in attributes.iter().enumerate()
        {
            buffers[i] = binding.buffer.buffer;
            offsets_in_bytes[i] = binding.offset_in_bytes;
        }
        unsafe { self.record.buffer.pool.device.logical_device.cmd_bind_vertex_buffers(self.record.buffer.command_buffer, first_binding, &buffers, &offsets_in_bytes); }
        self
    }

    #[inline]
    pub fn bind_descriptor_sets(&mut self, pipeline_layout: &PipelineLayout, descriptor_sets: &[&DescriptorSet]) -> &mut Self
    {
        self.record.bind_descriptor_sets_internal(pipeline_layout, descriptor_sets, vk::PipelineBindPoint::GRAPHICS);
        self
    }

    #[inline]
    pub fn push_constant<T>(&mut self, pipeline_layout: &PipelineLayout, push_constant: &T) -> &mut Self
    {
        self.record.push_constant(pipeline_layout, push_constant);
        self
    }

    #[inline]
    pub fn draw(&mut self, draw_mode: DrawMode) -> &mut Self
    {
        match draw_mode
        {
            DrawMode::Index { index_count, instance_count, first_index, vertex_offset, first_instance } => unsafe
            {
                self.record.buffer.pool.device.logical_device.cmd_draw_indexed(self.record.buffer.command_buffer, index_count, instance_count, first_index, vertex_offset, first_instance);
            },
            DrawMode::Vertex { vertex_count, instance_count, first_vertex, first_instance } => unsafe
            {
                self.record.buffer.pool.device.logical_device.cmd_draw(self.record.buffer.command_buffer, vertex_count, instance_count, first_vertex, first_instance);
            }
        };
        self
    }
}

impl Drop for CommandBufferRecordRenderPass<'_, '_>
{
    #[inline]
    fn drop(&mut self)
    {
        unsafe { self.record.buffer.pool.device.logical_device.cmd_end_render_pass(self.record.buffer.command_buffer); }
    }
}
