use super::*;

impl ImageLayout
{
    pub(crate) const fn vk_image_layout(&self, depth: bool) -> vk::ImageLayout
    {
        match self
        {
            ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
            ImageLayout::Attachment => if depth { vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL } else { vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL },
            ImageLayout::Shader => if depth { vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL } else { vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }
        }
    }
}

impl ImageChannelType
{
	pub(crate) const fn size_in_bytes(&self) -> u32
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

    pub(crate) const fn vk_format(&self) -> vk::Format
    {
        match self
        {
            ImageChannelType::BgraSrgb => vk::Format::B8G8R8A8_SRGB,
            ImageChannelType::BgraSnorm => vk::Format::B8G8R8A8_SNORM,
            ImageChannelType::BgraUnorm => vk::Format::B8G8R8A8_UNORM,
            ImageChannelType::BgraSint => vk::Format::B8G8R8A8_SINT,
            ImageChannelType::BgraUint => vk::Format::B8G8R8A8_UINT,
            ImageChannelType::RSrgb => vk::Format::R8_SRGB,
            ImageChannelType::RSnorm => vk::Format::R8_SNORM,
            ImageChannelType::RUnorm => vk::Format::R8_UNORM,
            ImageChannelType::RSint => vk::Format::R8_SINT,
            ImageChannelType::RUint => vk::Format::R8_UINT,
            ImageChannelType::DSfloat => vk::Format::D32_SFLOAT
        }
    }

    pub(crate) const fn has_depth(&self) -> bool
    {
        match self
        {
            ImageChannelType::DSfloat => true,
            _ => false
        }
    }
}

impl ImageType
{
    pub(crate) const fn layers(&self) -> u32
    {
        if let Some(layers) = self.layers { layers } else { 1 }
    }

    pub(crate) const fn layer_size_in_bytes(&self) -> u64
    {
        (self.channel.size_in_bytes() * self.width * self.height) as u64
    }
}

impl Msaa
{
    pub(crate) const fn vk_sample_count(&self) -> vk::SampleCountFlags
    {
        match self
        {
            Msaa::None => vk::SampleCountFlags::TYPE_1,
            Msaa::X2 => vk::SampleCountFlags::TYPE_2,
            Msaa::X4 => vk::SampleCountFlags::TYPE_4
        }
    }
}

impl ImageUsage
{
    pub(crate) fn vk_image_usage_flags(&self) -> vk::ImageUsageFlags
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

    pub(crate) fn mip_levels(&self, image_type: ImageType) -> u32
    {
        if let ImageUsage::Texture { mipmapping: true } = self { (u32::max(image_type.width, image_type.height) as f32).log2().floor() as u32 + 1 } else { 1 }
    }

    pub(crate) const fn depth(&self) -> bool
    {
        match self
        {
            ImageUsage::Attachment { depth: true, .. } => true,
             _ => false
        }
    }

    pub(crate) const fn vk_sample_count(&self) -> vk::SampleCountFlags
    {
        if let ImageUsage::Attachment { samples, .. } = self { samples.vk_sample_count() }
        else { vk::SampleCountFlags::TYPE_1 }
    }
}

impl Image
{
    pub fn ty(&self) -> ImageType { self.image_type }
}

impl SamplerFilter
{
    pub(crate) const fn vk_filter(&self) -> vk::Filter
    {
        match self
        {
            SamplerFilter::Linear => vk::Filter::LINEAR,
            SamplerFilter::Nearest => vk::Filter::NEAREST
        }
    }

    pub(crate) const fn vk_sampler_mipmap_mode(&self) -> vk::SamplerMipmapMode
    {
        match self
        {
            SamplerFilter::Linear => vk::SamplerMipmapMode::LINEAR,
            SamplerFilter::Nearest => vk::SamplerMipmapMode::NEAREST
        }
    }
}

impl SamplerAddressMode
{
    pub(crate) const fn vk_sampler_addres_mode(&self) -> vk::SamplerAddressMode
    {
        match self
        {
            SamplerAddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            SamplerAddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            SamplerAddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE
        }
    }
}
