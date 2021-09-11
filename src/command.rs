use super::*;

impl Device
{
    pub fn new_command_pool(&self, queue_family: &QueueFamily) -> CommandPool
    {
        let command_pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family.index as u32)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { self.0.logical_device.create_command_pool(&command_pool_info, None) }.unwrap();
        CommandPool { device: self.0.clone(), pool: command_pool, queue_family_index: queue_family.index, queue_family_flags: queue_family.flags }
    }
}

impl CommandPool
{
    pub fn new_command_buffer(&self) -> CommandBuffer
    {
        let command_bufffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .command_buffer_count(1);
        let command_buffer = unsafe { self.device.logical_device.allocate_command_buffers(&command_bufffer_allocate_info) }.unwrap()[0];
        CommandBuffer { pool: self, command_buffer }
    }
}

pub enum DrawMode<'a>
{
    Index { binding: IndexBinding<'a>, offset: u32, count: u32 },
    Vertex { offset: u32, count: u32 }
}

impl<'a> CommandBuffer<'a>
{
    #[inline]
    pub fn record<'b>(&'b mut self) -> CommandBufferRecord<'a, 'b>
    {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::empty());
        unsafe { self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info) }.unwrap();
        CommandBufferRecord { buffer: self }
    }

    #[inline]
    pub fn submit(&self, queue: &Queue, wait: &Semaphore, signal: &Semaphore, mark: &Fence)
    {
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::submit: Wrong queue family."); }
        let submit_info =
        [   vk::SubmitInfo::builder()
            .wait_semaphores(std::slice::from_ref(&wait.semaphore))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(std::slice::from_ref(&self.command_buffer))
            .signal_semaphores(std::slice::from_ref(&signal.semaphore))
            .build()
        ];
        unsafe { self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, mark.fence) }.unwrap();
    }
}

pub struct CommandBufferRecord<'a, 'b>
{
    buffer: &'b mut CommandBuffer<'a>
}
            
impl<'a, 'b> CommandBufferRecord<'a, 'b>
{
    #[inline]
    pub fn render_pass<'c>(&'c mut self, render_pass: &RenderPass, framebuffer: &Framebuffer) -> CommandBufferRecordRenderPass<'a, 'b, 'c>
    {
        let (width, height) = framebuffer.size;
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
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
        if DEBUG_MODE { if let ImageUsage::Attachment { depth: _, sampled: false } = image.image_usage { panic!("CommandBufferRecord::pipeline_barrier: This attachment cannot be sampled."); } }
        let depth = image.image_usage.depth();
        let barrier = vk::ImageMemoryBarrier::builder()
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

impl Drop for CommandBufferRecord<'_, '_>
{
    #[inline]
    fn drop(&mut self)
    {
        unsafe { self.buffer.pool.device.logical_device.end_command_buffer(self.buffer.command_buffer) }.unwrap();
    }
}

pub struct CommandBufferRecordRenderPass<'a, 'b, 'c>
{
    record: &'c mut CommandBufferRecord<'a, 'b>
}

impl<'a, 'b, 'c> CommandBufferRecordRenderPass<'a, 'b, 'c>
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
    pub fn set_view(&mut self, view_info: &ViewInfo) -> &mut Self
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
    pub fn bind_attributes<const N: usize>(&mut self, attributes: [&AttributeBinding; N]) -> &mut Self
    {
        let (mut buffers, mut offsets_in_bytes) = ([Default::default(); N], [Default::default(); N]);
        for (i, binding) in attributes.iter().enumerate()
        {
            buffers[i] = binding.buffer.buffer;
            offsets_in_bytes[i] = binding.offset_in_bytes;
        }
        unsafe { self.record.buffer.pool.device.logical_device.cmd_bind_vertex_buffers(self.record.buffer.command_buffer, 0, &buffers, &offsets_in_bytes); }
        self
    }

    #[inline]
    pub fn bind_descriptor_sets(&mut self, pipeline_layout: &PipelineLayout, descriptor_sets: &[&DescriptorSet]) -> &mut Self
    {
        for set in descriptor_sets
        {
            unsafe
            {
                self.record.buffer.pool.device.logical_device.cmd_bind_descriptor_sets
                (
                    self.record.buffer.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
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
    pub fn push_constant<T>(&mut self, pipeline_layout: &PipelineLayout, push_constant: &T) -> &mut Self
    {
        let (shader_stages, size) = pipeline_layout.push_constant.expect("CommandBufferRecordRenderPass::push_constant: This layout has no push constant.");
        if DEBUG_MODE && std::mem::size_of::<T>() as u32 != size { panic!("CommandBufferRecordRenderPass::push_constant: Incompatible data size."); }
        unsafe
        {
            let data = std::slice::from_raw_parts(push_constant as *const T as *const u8, std::mem::size_of::<T>());
            self.record.buffer.pool.device.logical_device.cmd_push_constants(self.record.buffer.command_buffer, pipeline_layout.layout, shader_stages, 0, data);
        }
        self
    }

    #[inline]
    pub fn draw(&mut self, draw_mode: &DrawMode, instance_count: u32) -> &mut Self
    {
        match draw_mode
        {
            DrawMode::Index { binding, offset, count } => unsafe
            {
                self.record.buffer.pool.device.logical_device.cmd_bind_index_buffer(self.record.buffer.command_buffer, binding.buffer.buffer, binding.offset_in_bytes, binding.format);
                self.record.buffer.pool.device.logical_device.cmd_draw_indexed(self.record.buffer.command_buffer, *count, instance_count, *offset, 0, 0);
            },
            DrawMode::Vertex { offset, count } => unsafe
            {
                self.record.buffer.pool.device.logical_device.cmd_draw(self.record.buffer.command_buffer, *count, instance_count, *offset, 0);
            }
        };
        self
    }
}

impl Drop for CommandBufferRecordRenderPass<'_, '_, '_>
{
    #[inline]
    fn drop(&mut self)
    {
        unsafe { self.record.buffer.pool.device.logical_device.cmd_end_render_pass(self.record.buffer.command_buffer); }
    }
}
