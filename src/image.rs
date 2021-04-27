use super::*;

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
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
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

impl ImageUsage
{
    fn vk_image_usage_flags(&self) -> vk::ImageUsageFlags
    {
        match self
        {
            ImageUsage::Texture { mipmapping } =>
            {
                let flags = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED;
                if *mipmapping { flags | vk::ImageUsageFlags::TRANSFER_SRC } else { flags }
            },
            ImageUsage::Attachment { depth, texture, .. } =>
            {
                let flags = if *depth { vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT } else { vk::ImageUsageFlags::COLOR_ATTACHMENT };
                if *texture { flags | vk::ImageUsageFlags::SAMPLED } else { flags | vk::ImageUsageFlags::INPUT_ATTACHMENT }
            }
        }
    }

    fn mip_levels(&self, image_type: ImageType) -> u32
    {
        if let ImageUsage::Texture { mipmapping: true } = self { (u32::max(image_type.width, image_type.height) as f32).log2().floor() as u32 + 1 } else { 1 }
    }
}

pub enum SamplerFilter
{
    Linear,
    Nearest
}

impl SamplerFilter
{
    fn vk_filter(&self) -> vk::Filter
    {
        match self
        {
            SamplerFilter::Linear => vk::Filter::LINEAR,
            SamplerFilter::Nearest => vk::Filter::NEAREST
        }
    }

    fn vk_sampler_mipmap_mode(&self) -> vk::SamplerMipmapMode
    {
        match self
        {
            SamplerFilter::Linear => vk::SamplerMipmapMode::LINEAR,
            SamplerFilter::Nearest => vk::SamplerMipmapMode::NEAREST
        }
    }
}

pub enum SamplerAddressMode
{
    Repeat,
    MirroredRepeat,
    ClampToEdge
}

impl SamplerAddressMode
{
    fn vk_sampler_addres_mode(&self) -> vk::SamplerAddressMode
    {
        match self
        {
            SamplerAddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            SamplerAddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            SamplerAddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE
        }
    }
}

pub struct SamplerInfo
{
    pub mag_filter: SamplerFilter,
    pub min_filter: SamplerFilter,
    pub mipmap_filter: SamplerFilter,
    pub address_mode: SamplerAddressMode
}

impl ImageChannelType
{
	const fn size_in_bytes(&self) -> u32
	{
		match self
		{
			ImageChannelType::BgraSrgb => 4,
            ImageChannelType::BgraSnorm => 4,
            ImageChannelType::BgraUnorm => 4,
            ImageChannelType::BgraSint => 4,
            ImageChannelType::BgraUint => 4,
            ImageChannelType::RSrgb => 1,
            ImageChannelType::RSnorm => 1,
            ImageChannelType::RUnorm => 1,
            ImageChannelType::RSint => 1,
            ImageChannelType::RUint => 1,
            ImageChannelType::DSfloat => 4
		}
	}
}

impl ImageType
{
    fn layers(&self) -> u32
    {
        if let Some(layers) = self.layers { layers } else { 1 }
    }

	fn layer_size_in_bytes(&self) -> u64
	{
		(self.channel.size_in_bytes() * self.width * self.height) as u64
	}
}

impl Image
{
    pub fn ty(&self) -> ImageType { self.image_type }
}

impl ImageBuffer
{
	pub fn write(&mut self, data: &[u8])
	{
		if data.len() as u64 != self.image_type.layer_size_in_bytes() { panic!("ImageBuffer::write: Incompatible buffer size."); }
		let buffer_ptr = self.device.allocator.map_memory(&self.allocation).unwrap();
        unsafe { buffer_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len()); }
        self.device.allocator.unmap_memory(&self.allocation).unwrap();
	}
}

impl<'a> CommandBuffer<'a>
{
    pub fn copy_image<'b, 'c>(self, queue: &Queue, src: &'b ImageBuffer, dst: &'c Image, layer: u32, mark: Fence) -> CopyImageFence<'a, 'b, 'c>
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
                barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;
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
                barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
                barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
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
        CopyImageFence { mark, _command_buffer: self, _src: src, _dst: dst }
    }
}

pub struct CopyImageFence<'a, 'b, 'c>
{
    pub mark: Fence,
    _command_buffer: CommandBuffer<'a>,
    _src: &'b ImageBuffer,
    _dst: &'c Image
}
