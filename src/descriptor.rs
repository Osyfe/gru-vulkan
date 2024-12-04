use super::*;

impl Device
{
	pub fn new_descriptor_set_layout(&self, set: u32, bindings: Vec<DescriptorBindingInfo>) -> DescriptorSetLayout
    {
        let descriptor_set_layout_bindings: Vec<_> = bindings.iter().enumerate().map(|(id, binding)|
        {
            let stage_flags =
                if binding.vertex { vk::ShaderStageFlags::VERTEX } else { vk::ShaderStageFlags::empty() }
              | if binding.fragment { vk::ShaderStageFlags::FRAGMENT } else { vk::ShaderStageFlags::empty() };
            vk::DescriptorSetLayoutBinding::default()
                .binding(id as u32)
                .descriptor_type(binding.vk_type())
                .descriptor_count(binding.count)
                .stage_flags(stage_flags)
        }).collect();
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_set_layout_bindings);
        let descriptor_set_layout = unsafe { self.0.logical_device.create_descriptor_set_layout(&descriptor_set_layout_info, None) }.unwrap();
        DescriptorSetLayout(Arc::new(RawDescriptorSetLayout { device: self.0.clone(), set, bindings: Box::from(bindings), descriptor_set_layout }))
    }

    pub fn new_descriptor_sets(&self, set_layouts: &[(&DescriptorSetLayout, u32)]) -> Vec<Vec<DescriptorSet>>
    {
        let (mut set_count, mut struct_count, mut sampler_count, mut input_attachment_count) = (0, 0, 0, 0);
        for (layout, count) in set_layouts
        {
            set_count += count;
            let (a, b, c) = layout.0.type_count();
            struct_count += count * a;
            sampler_count += count * b;
            input_attachment_count += count * c;
        }
        
        let mut pool_sizes = vec![];
        if struct_count > 0
        {
            pool_sizes.push(vk::DescriptorPoolSize
            {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: struct_count,
            })
        };
        if sampler_count > 0
        {
            pool_sizes.push(vk::DescriptorPoolSize
            {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: sampler_count,
            })
        };
        if input_attachment_count > 0
        {
            pool_sizes.push(vk::DescriptorPoolSize
            {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: input_attachment_count,
            })
        };

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(set_count)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { self.0.logical_device.create_descriptor_pool(&descriptor_pool_info, None) }.unwrap();
        let pool_arc = Arc::new(DescriptorPool
        {
            device: self.0.clone(),
            pool: descriptor_pool
        });
        
        set_layouts.iter().map(|(layout, count)|
        {
            let mut layouts = Vec::with_capacity(*count as usize);
            for _ in 0..*count { layouts.push(layout.0.descriptor_set_layout); }
            let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts[..]);
            let descriptor_sets = unsafe { self.0.logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info) }.unwrap();
            descriptor_sets.iter().map(|set| DescriptorSet { pool: pool_arc.clone(), descriptor_set: *set, layout: layout.0.clone() }).collect()
        }).collect()
    }
}

impl DescriptorBindingInfo
{
    pub fn from_struct<T: DescriptorStructReprC>(count: u32, vertex: bool, fragment: bool) -> Self
    {
        Self
        {
            ty: DescriptorBindingType::Struct { size_in_bytes: std::mem::size_of::<T>() as u32 },
            count, vertex, fragment
        }
    }

    pub fn from_sampler(image_channel_type: ImageChannelType, count: u32, vertex: bool, fragment: bool) -> Self
    {
    	Self
    	{
            ty: DescriptorBindingType::Sampler { image_channel_type },
    		count, vertex, fragment
    	}
    }

    pub fn from_input_attachment(image_channel_type: ImageChannelType) -> Self
    {
        Self
        {
            ty: DescriptorBindingType::SubpassInput { image_channel_type },
            count: 1, vertex: false, fragment: true
        }
    }

    fn vk_type(&self) -> vk::DescriptorType
    {
        match self.ty
        {
            DescriptorBindingType::Struct { .. } => vk::DescriptorType::UNIFORM_BUFFER,
            DescriptorBindingType::Sampler { .. } => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorBindingType::SubpassInput { .. } => vk::DescriptorType::INPUT_ATTACHMENT
        }
    }

    fn type_count(&self) -> (u32, u32, u32)
    {
        match self.ty
        {
            DescriptorBindingType::Struct { .. } => (self.count, 0, 0),
            DescriptorBindingType::Sampler { .. } => (0, self.count, 0),
            DescriptorBindingType::SubpassInput { .. } => (0, 0, 1)
        }
    }
}

impl RawDescriptorSetLayout
{
    fn type_count(&self) -> (u32, u32, u32)
    {
        let (mut struct_count, mut sampler_count, mut input_attachment_count) = (0, 0, 0);
        for binding in self.bindings.iter()
        {
            let (a, b, c) = binding.type_count();
            struct_count += a;
            sampler_count += b;
            input_attachment_count += c;
        }
        (struct_count, sampler_count, input_attachment_count)
    }
}

impl DescriptorSet
{
    pub fn update_struct<T: DescriptorStructReprC>(&mut self, binding: u32, buffer: &Buffer, view: &BufferView<T>)
    {
        if DEBUG_MODE && view.layout_id != buffer.layout_id { panic!("DescriptorSet::update_struct: Incompatible BufferView"); }
        let layout = &self.layout.bindings[binding as usize];
        if view.count != layout.count { panic!("DescriptorSet::update_struct: Wrong amount of uniforms: {} vs {}.", view.count, layout.count); }
        match layout.ty
        {
        	DescriptorBindingType::Struct { size_in_bytes } => if std::mem::size_of::<T>() != size_in_bytes as usize { panic!("DescriptorSet::update_struct: Incompatible struct size."); },
            DescriptorBindingType::Sampler { .. } => panic!("DescriptorSet::update_struct: Incompatible DescriptorBindingType."),
            DescriptorBindingType::SubpassInput { .. } => panic!("DescriptorSet::update_struct: Incompatible DescriptorBindingType.")
        };
        let buffer_infos: Vec<_> = (0..layout.count).map(|i| vk::DescriptorBufferInfo 
        {
            buffer: buffer.buffer,
            offset: (view.offset_in_bytes as u32 + i * view.stride) as u64,
            range: view.stride as u64,
        }).collect();
        let descriptor_sets_write =
        [
            vk::WriteDescriptorSet::default()
                .dst_set(self.descriptor_set)
                .dst_binding(binding)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
        ];
        unsafe { self.pool.device.logical_device.update_descriptor_sets(&descriptor_sets_write, &[]) };
    }

    pub fn update_sampler(&mut self, binding: u32, images: &[&Image], sampler: &Sampler)
    {
        let layout = &self.layout.bindings[binding as usize];
        if images.len() as u32 != layout.count { panic!("DescriptorSet::update_sampler: Wrong amount of images: {} vs {}.", images.len(), layout.count); }
        match layout.ty
        {
        	DescriptorBindingType::Struct { .. } => panic!("DescriptorSet::update_sampler: Incompatible DescriptorBindingType."),
            DescriptorBindingType::Sampler { image_channel_type } => for (i, image) in images.iter().enumerate()
            {
                if image.image_type.channel != image_channel_type { panic!("DescriptorSet::update_sampler: Incompatible ImageType for image {}.", i); }
            },
            DescriptorBindingType::SubpassInput { .. } => panic!("DescriptorSet::update_struct: Incompatible DescriptorBindingType.")
        };
        let image_infos: Vec<_> = images.iter().map(|image|
        {
            if let ImageUsage::Attachment { texture: false, .. } = image.image_usage { panic!("DescriptorSet::update_sampler: This attachment cannot be sampled."); }
            vk::DescriptorImageInfo
            {
                image_layout: ImageLayout::Shader.vk_image_layout(image.image_usage.depth()),
                image_view: image.image_view,
                sampler: sampler.sampler,
                ..Default::default()
            }
        }).collect();
        let descriptor_write_image = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptor_set)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos);
        unsafe { self.pool.device.logical_device.update_descriptor_sets(&[descriptor_write_image], &[]); }
    }

    pub fn update_input_attachment(&mut self, binding: u32, image: &Image)
    {
        let layout = &self.layout.bindings[binding as usize];
        match layout.ty
        {
            DescriptorBindingType::Struct { .. } => panic!("DescriptorSet::update_input_attachment: Incompatible DescriptorBindingType."),
            DescriptorBindingType::Sampler { .. } => panic!("DescriptorSet::update_input_attachment: Incompatible DescriptorBindingType."),
            DescriptorBindingType::SubpassInput { image_channel_type } =>
                if image.image_type.channel != image_channel_type { panic!("DescriptorSet::update_input_attachment: Incompatible ImageType."); }
        };
        let input_attachment_info = vk::DescriptorImageInfo
        {
            image_layout: ImageLayout::Shader.vk_image_layout(image.image_usage.depth()),
            image_view: image.image_view,
            ..Default::default()
        };
        let descriptor_write_image = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptor_set)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
            .image_info(std::slice::from_ref(&input_attachment_info));
        unsafe { self.pool.device.logical_device.update_descriptor_sets(&[descriptor_write_image], &[]); }
    }
}
