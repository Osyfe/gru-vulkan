use super::*;

mod stuff;
pub use stuff::*;

impl Device
{
	pub fn new_image(&self, image_type: ImageType, image_usage: ImageUsage) -> Image
    {
        if DEBUG_MODE && image_usage.depth() && !image_type.channel.has_depth() { panic!("Device::new_image: This ImageChannelType has no depth component."); }
        let mip_levels = image_usage.mip_levels(image_type);
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D
            {
                width: image_type.width,
                height: image_type.height,
                depth: 1,
            })
            .mip_levels(mip_levels)
            .array_layers(image_type.layers())
            .format(image_type.channel.vk_format())
            .tiling(vk::ImageTiling::OPTIMAL)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(image_usage.vk_sample_count())
            .usage(image_usage.vk_image_usage_flags());
        let allocation_create_info = vk_mem::AllocationCreateInfo
        {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };
        let (vk_image, allocation, allocation_info) = self.0.allocator.create_image(&image_create_info, &allocation_create_info).unwrap();
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(vk_image)
            .view_type(if image_type.layers.is_some() { vk::ImageViewType::TYPE_2D_ARRAY } else { vk::ImageViewType::TYPE_2D })
            .format(image_type.channel.vk_format())
            .subresource_range(vk::ImageSubresourceRange
            {
                aspect_mask: if image_usage.depth() { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::COLOR },
                level_count: mip_levels,
                layer_count: image_type.layers(),
                ..Default::default()
            });
        let image_view = unsafe { self.0.logical_device.create_image_view(&image_view_create_info, None) }.unwrap();
        Image
        {
            device: self.0.clone(),
            allocation,
            _allocation_info: allocation_info,
            image: vk_image,
            image_view,
            image_type,
            mip_levels,
            image_usage
        }
    }

    pub fn new_image_buffer(&self, image_type: ImageType) -> ImageBuffer
    {
        let memory_usage_flags = vk_mem::MemoryUsage::CpuOnly;
        let allocation_create_info = vk_mem::AllocationCreateInfo { usage: memory_usage_flags, ..Default::default() };
        let (buffer, allocation, allocation_info) = self.0.allocator.create_buffer
        (
            &ash::vk::BufferCreateInfo::builder()
                .size(image_type.layer_size_in_bytes())
                .usage(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST)
                .build(),
            &allocation_create_info,
        ).unwrap();
        ImageBuffer
        {
            device: self.0.clone(),
            allocation,
            _allocation_info: allocation_info,
            buffer,
            image_type
        }
    }

    pub fn new_sampler(&self, info: &SamplerInfo) -> Sampler
    {
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(info.mag_filter.vk_filter())
            .min_filter(info.min_filter.vk_filter())
            .mipmap_mode(info.mipmap_filter.vk_sampler_mipmap_mode())
            .min_lod(0.0)
            .max_lod(100.0)
            .mip_lod_bias(0.0) //TODO ?
            .address_mode_u(info.address_mode.vk_sampler_addres_mode())
            .address_mode_v(info.address_mode.vk_sampler_addres_mode())
            .address_mode_w(info.address_mode.vk_sampler_addres_mode())
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .unnormalized_coordinates(false)
            .compare_enable(false); //TODO for shadow maps?
        let sampler = unsafe { self.0.logical_device.create_sampler(&sampler_info, None) }.unwrap();
        Sampler { device: self.0.clone(), sampler }
    }
}

impl ImageBuffer
{
    pub fn size(&self) -> usize
    {
        self.image_type.layer_size_in_bytes() as usize
    }

	pub fn write(&mut self, data: &[u8])
	{
		if data.len() as u64 != self.image_type.layer_size_in_bytes() { panic!("ImageBuffer::write: Incompatible buffer size."); }
		let buffer_ptr = self.device.allocator.map_memory(&self.allocation).unwrap();
        unsafe { buffer_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len()); }
        self.device.allocator.unmap_memory(&self.allocation).unwrap();
	}

    pub fn read(&mut self, data: &mut [u8])
    {
        if data.len() as u64 != self.image_type.layer_size_in_bytes() { panic!("ImageBuffer::write: Incompatible buffer size."); }
        let buffer_ptr = self.device.allocator.map_memory(&self.allocation).unwrap();
        unsafe { data.as_mut_ptr().copy_from_nonoverlapping(buffer_ptr, data.len()); }
        self.device.allocator.unmap_memory(&self.allocation).unwrap();
    }
}

impl<'a> CommandBuffer<'a>
{
    pub fn copy_to_image<'b, 'c>(self, queue: &Queue, src: &'b ImageBuffer, dst: &'c Image, layer: u32, mark: Fence) -> CopyImageFence<'a, 'b, 'c>
    {
        if DEBUG_MODE { if let ImageUsage::Attachment { .. } = dst.image_usage { panic!("CommandBuffer::copy_image: Cannot transfer to framebuffer."); } }
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::copy_image: Wrong queue family."); }
        if DEBUG_MODE && 
          (!self.pool.queue_family_flags.contains(vk::QueueFlags::TRANSFER)
        || !self.pool.queue_family_flags.contains(vk::QueueFlags::GRAPHICS))
            { panic!("CommandBuffer::copy_image: This queue family does not support graphic transfer operations."); }
        if DEBUG_MODE && src.image_type != dst.image_type { panic!("CommandBuffer::copy_image: Buffer and image need to have the same image_type."); }
        if DEBUG_MODE && layer >= dst.image_type.layers() { panic!("CommandBuffer::copy_image: Layer too large."); }
        let image_type = src.image_type;

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let mut barrier = vk::ImageMemoryBarrier::builder()
            .image(dst.image)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .subresource_range(vk::ImageSubresourceRange
            {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: dst.mip_levels,
                base_array_layer: layer,
                layer_count: 1,
            })
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .build();
        let image_subresource = vk::ImageSubresourceLayers
        {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: layer,
            layer_count: 1,
        };
        let region = vk::BufferImageCopy
        {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D
            {
                width: image_type.width,
                height: image_type.height,
                depth: 1,
            },
            image_subresource,
            ..Default::default()
        };
        let submit_info =
        [   vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&self.command_buffer))
            .build()
        ];
        unsafe
        {
            self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info).unwrap();
            self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
            self.pool.device.logical_device.cmd_copy_buffer_to_image(self.command_buffer, src.buffer, dst.image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]);
            let mut mip_width = dst.image_type.width;
            let mut mip_height = dst.image_type.height;
            barrier.subresource_range.level_count = 1;
            for i in 1..dst.mip_levels
            {
                barrier.subresource_range.base_mip_level = i - 1;
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;
                barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
                let image_blit = vk::ImageBlit::builder()
                    .src_offsets(
                    [
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D { x: mip_width as i32, y: mip_height as i32, z: 1 }
                    ])
                    .src_subresource(vk::ImageSubresourceLayers
                    {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: i - 1,
                        base_array_layer: layer,
                        layer_count: 1
                    })
                    .dst_offsets(
                    [
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D { x: if mip_width > 1 { mip_width as i32 / 2 } else { 1 }, y: if mip_height > 1 { mip_height as i32 / 2 } else { 1 }, z: 1 },
                    ])
                    .dst_subresource(vk::ImageSubresourceLayers
                    {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: i,
                        base_array_layer: layer,
                        layer_count: 1
                    })
                    .build();
                self.pool.device.logical_device.cmd_blit_image(self.command_buffer, dst.image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst.image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[image_blit], vk::Filter::LINEAR);
                barrier.subresource_range.base_mip_level = i - 1;
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
                barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
                barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
                if mip_width > 1 { mip_width /= 2; }
                if mip_height > 1 { mip_height /= 2; }
            }
            barrier.subresource_range.base_mip_level = dst.mip_levels - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
            self.pool.device.logical_device.end_command_buffer(self.command_buffer).unwrap();
            self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, mark.fence).unwrap();
        }
        CopyImageFence { mark, command_buffer: self, _buf: src, _img: &dst.image }
    }

    pub fn copy_from_image<'b, 'c>(self, queue: &Queue, src: CopyImageSource<'c>, dst: &'b ImageBuffer, mark: Fence) -> CopyImageFence<'a, 'b, 'c>
    {
        let (legal, image, image_type, layout) = match src
        {
            CopyImageSource::Swapchain(image) =>
                (true, image.image, ImageType { channel: Swapchain::IMAGE_CHANNEL_TYPE, width: image.width, height: image.height, layers: None }, vk::ImageLayout::PRESENT_SRC_KHR),
            CopyImageSource::Image(image) =>
                if let ImageUsage::Attachment { transfer_src: true, .. } = image.image_usage { (true, &image.image, image.image_type, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) }
                else { (false, &image.image, image.image_type, vk::ImageLayout::UNDEFINED) }
        };
        if DEBUG_MODE && !legal { panic!("CommandBuffer::copy_image: Cannot transfer from this image."); }
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::copy_image: Wrong queue family."); }
        if DEBUG_MODE && 
          (!self.pool.queue_family_flags.contains(vk::QueueFlags::TRANSFER)
        || !self.pool.queue_family_flags.contains(vk::QueueFlags::GRAPHICS))
            { panic!("CommandBuffer::copy_image: This queue family does not support graphic transfer operations."); }
        if DEBUG_MODE && image_type != dst.image_type { panic!("CommandBuffer::copy_image: Buffer and image need to have the same image_type."); }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let mut barrier = vk::ImageMemoryBarrier::builder()
            .image(*image)
            .src_access_mask(vk::AccessFlags::MEMORY_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .old_layout(layout)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .subresource_range(vk::ImageSubresourceRange
            {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .build();
        let image_subresource = vk::ImageSubresourceLayers
        {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        };
        let region = vk::BufferImageCopy
        {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D
            {
                width: image_type.width,
                height: image_type.height,
                depth: 1,
            },
            image_subresource,
            ..Default::default()
        };
        let submit_info =
        [   vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&self.command_buffer))
            .build()
        ];
        unsafe
        {
            self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info).unwrap();
            self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
            self.pool.device.logical_device.cmd_copy_image_to_buffer(self.command_buffer, *image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst.buffer, &[region]);
            
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::MEMORY_READ;
            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = layout;
            
            self.pool.device.logical_device.cmd_pipeline_barrier(self.command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier]);
            self.pool.device.logical_device.end_command_buffer(self.command_buffer).unwrap();
            self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, mark.fence).unwrap();
        }
        CopyImageFence { mark, command_buffer: self, _buf: dst, _img: image }
    }
}

pub enum CopyImageSource<'a>
{
    Swapchain(SwapchainImage<'a>),
    Image(&'a Image)
}

pub struct CopyImageFence<'a, 'b, 'c>
{
    pub mark: Fence,
    pub command_buffer: CommandBuffer<'a>,
    _buf: &'b ImageBuffer,
    _img: &'c vk::Image
}

pub enum SamplerFilter
{
    Linear,
    Nearest
}

pub enum SamplerAddressMode
{
    Repeat,
    MirroredRepeat,
    ClampToEdge
}

pub struct SamplerInfo
{
    pub mag_filter: SamplerFilter,
    pub min_filter: SamplerFilter,
    pub mipmap_filter: SamplerFilter,
    pub address_mode: SamplerAddressMode
}