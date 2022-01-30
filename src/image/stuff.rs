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
			Self::BgraSrgb => 4,
            Self::BgraSnorm => 4,
            Self::BgraUnorm => 4,
            Self::BgraSint => 4,
            Self::BgraUint => 4,
            Self::RSrgb => 1,
            Self::RSnorm => 1,
            Self::RUnorm => 1,
            Self::RSint => 1,
            Self::RUint => 1,
            Self::R32Uint => 4,
            Self::RSfloat => 4,
            Self::DSfloat => 4
		}
	}

    pub(crate) const fn vk_format(&self) -> vk::Format
    {
        match self
        {
            Self::BgraSrgb => vk::Format::B8G8R8A8_SRGB,
            Self::BgraSnorm => vk::Format::B8G8R8A8_SNORM,
            Self::BgraUnorm => vk::Format::B8G8R8A8_UNORM,
            Self::BgraSint => vk::Format::B8G8R8A8_SINT,
            Self::BgraUint => vk::Format::B8G8R8A8_UINT,
            Self::RSrgb => vk::Format::R8_SRGB,
            Self::RSnorm => vk::Format::R8_SNORM,
            Self::RUnorm => vk::Format::R8_UNORM,
            Self::RSint => vk::Format::R8_SINT,
            Self::RUint => vk::Format::R8_UINT,
            Self::R32Uint => vk::Format::R32_UINT,
            Self::RSfloat => vk::Format::R32_SFLOAT,
            Self::DSfloat => vk::Format::D32_SFLOAT
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
            ImageUsage::Attachment { depth, texture, transfer_src, .. } =>
            {
                let flags =
                    if *depth { vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT } else { vk::ImageUsageFlags::COLOR_ATTACHMENT }
                  | if *texture { vk::ImageUsageFlags::SAMPLED } else { vk::ImageUsageFlags::INPUT_ATTACHMENT };
                if *transfer_src { flags | vk::ImageUsageFlags::TRANSFER_SRC } else { flags }
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
