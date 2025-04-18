use super::*;
use std::ops::Range;

impl Device
{
	pub fn new_render_pass(&self, info: RenderPassInfo) -> RenderPass
    {
        let RenderPassInfo
        {
            color_attachments,
            depth_attachment,
            subpasses,
            #[cfg(feature = "multiview")]
            multiview
        } = info;
        //attachments
        let num_attachments = color_attachments.len() + depth_attachment.map_or_else(|| 0, |_| 1);
        let mut clear_colors = Vec::with_capacity(num_attachments);
        let mut attachments = Vec::with_capacity(num_attachments);
        let mut depth_attachment_index = None;
        for attachment in color_attachments
        {
            let attachment = match attachment
            {
                RenderPassColorAttachment::Swapchain(load) =>
                {  
                    clear_colors.push(load.vk_clear_value());
                    vk::AttachmentDescription::default()
                        .format(Swapchain::IMAGE_CHANNEL_TYPE.vk_format())
                        .load_op(load.vk_attachment_load_op())
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .samples(vk::SampleCountFlags::TYPE_1)
                },
                RenderPassColorAttachment::Image { image_channel_type, samples, load, store, initial_layout, final_layout } =>
                {
                    clear_colors.push(load.vk_clear_value());
                    vk::AttachmentDescription::default()
                        .format(image_channel_type.vk_format())
                        .load_op(load.vk_attachment_load_op())
                        .store_op(store.vk_attachment_store_op())
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(initial_layout.vk_image_layout(false))
                        .final_layout(final_layout.vk_image_layout(false))
                        .samples(samples.vk_sample_count())
                }
            };
            attachments.push(attachment);
        }
        if let Some(depth_attachment) = depth_attachment
        {
            if DEBUG_MODE && !depth_attachment.image_channel_type.has_depth() { panic!("Device::new_render_pass: This ImageChannelType hat no depth component."); }
            clear_colors.push(depth_attachment.load.vk_clear_value());
            attachments.push(
            {
                vk::AttachmentDescription::default()
                    .format(depth_attachment.image_channel_type.vk_format())
                    .load_op(depth_attachment.load.vk_attachment_load_op())
                    .store_op(depth_attachment.store.vk_attachment_store_op())
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(depth_attachment.initial_layout.vk_image_layout(true))
                    .final_layout(depth_attachment.final_layout.vk_image_layout(true))
                    .samples(depth_attachment.samples.vk_sample_count())
            });
            depth_attachment_index = Some(attachments.len() as u32 - 1);
        }
        //subpasses
        struct SubpassReferences
        {
            input: Option<Range<usize>>,
            color: Option<Range<usize>>,
            resolve: Option<Range<usize>>,
            depth: Option<usize>
        }
        //vk::AttachmentReference needs to outlive vk::SubpassDescription but is copy -> we have to store unique structs until the end
        let mut all_input_references = Vec::with_capacity(attachments.len() * subpasses.len());
        let mut all_input_subpass_indices = Vec::with_capacity(attachments.len() * subpasses.len());
        let mut all_output_references = Vec::with_capacity(attachments.len() * subpasses.len());
        let mut all_output_subpass_indices = Vec::with_capacity(attachments.len() * subpasses.len());
        let mut all_resolve_references = Vec::with_capacity(attachments.len() * subpasses.len());
        let subpasses: Vec<_> = subpasses.iter().enumerate().map(|(subpass_index, subpass)|
        {
            //input
            let mut input_attachments_index = None;
            let mut input_attachments_count = 0;
            for (index, InputAttachment { fragment_input_attachment_index, attachment_index }) in subpass.input_attachments.iter().enumerate()
            {
                if DEBUG_MODE && *attachment_index >= attachments.len() as u32 { panic!("Device::new_render_pass: No attachment {}", attachment_index); }
                if DEBUG_MODE && *fragment_input_attachment_index != index as u32 { panic!("Wrong fragment_input_attachment_index: {} vs {}", fragment_input_attachment_index, index); }
                all_input_references.push(vk::AttachmentReference
                {
                    attachment: *attachment_index,
                    layout: ImageLayout::Shader.vk_image_layout(Some(index as u32) == depth_attachment_index),
                });
                all_input_subpass_indices.push(subpass_index as u32);
                if input_attachments_count == 0 { input_attachments_index = Some(all_input_references.len() - 1); }
                input_attachments_count += 1;
            }
            //color output
            let mut color_attachments_index = None;
            let mut color_attachments_count = 0;
            for (index, OutputAttachment { fragment_out_location, attachment_index }) in subpass.output_attachments.iter().enumerate()
            {
                if DEBUG_MODE && *attachment_index >= attachments.len() as u32 { panic!("Device::new_render_pass: No attachment {}", attachment_index); }
                if DEBUG_MODE && *fragment_out_location != index as u32 { panic!("Wrong fragment_out_location: {} vs {}", fragment_out_location, index); }
                all_output_references.push(vk::AttachmentReference
                {
                    attachment: *attachment_index,
                    layout: ImageLayout::Attachment.vk_image_layout(false),
                });
                all_output_subpass_indices.push(subpass_index as u32);
                if color_attachments_count == 0 { color_attachments_index = Some(all_output_references.len() - 1); }
                color_attachments_count += 1;
            }
            //resolve output
            let mut resolve_attachments_index = None;
            if let Some(resolve_attachments) = subpass.resolve_attachments
            {
            	if DEBUG_MODE && resolve_attachments.len() != subpass.output_attachments.len() { panic!("Device::new_render_pass: Same amount of resolve attachments as output attachments required."); }
            	for index in resolve_attachments.iter()
            	{
            		all_resolve_references.push(match index
            		{
            			ResolveAttachment::Index(index) =>
            			{
            				if DEBUG_MODE && *index >= color_attachments.len() as u32 { panic!("Device::new_render_pass: No attachment {}", index); }
            				vk::AttachmentReference
                			{
                    			attachment: *index,
                    			layout: ImageLayout::Attachment.vk_image_layout(false),
                			}
            			},
            			ResolveAttachment::Unused => vk::AttachmentReference
                		{
                    		attachment: vk::ATTACHMENT_UNUSED,
                    		layout: ImageLayout::Attachment.vk_image_layout(false),
                		}
            		});
            		if resolve_attachments_index == None { resolve_attachments_index = Some(all_resolve_references.len() - 1); }
            	}
            }
            //depth reference/output
            let mut depth_attachment = None;
            if subpass.depth_attachment
            {
                match depth_attachment_index
                {
                    Some(depth_attachment_index) =>
                    {
                        all_output_references.push(vk::AttachmentReference
                        {
                            attachment: depth_attachment_index,
                            layout: ImageLayout::Attachment.vk_image_layout(true),
                        });
                        all_output_subpass_indices.push(subpass_index as u32);
                        depth_attachment = Some(all_output_references.len() - 1);
                    },
                    None => panic!("Device::new_render_pass: No depth attachment.")
                }
            }
            //putting together
            SubpassReferences
            {
                input: input_attachments_index.map(|index| index..(index + input_attachments_count)),
                color: color_attachments_index.map(|index| index..(index + color_attachments_count)),
                resolve: resolve_attachments_index.map(|index| index..(index + color_attachments_count)),
                depth: depth_attachment
            }
        }).collect();
        let subpasses: Vec<_> = subpasses.into_iter().map(|subpass|
        {
            let mut subpass_description = vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

            if let Some(range) = subpass.input { subpass_description = subpass_description.input_attachments(&all_input_references[range]); }
            if let Some(range) = subpass.color { subpass_description = subpass_description.color_attachments(&all_output_references[range]); }
            if let Some(range) = subpass.resolve { subpass_description = subpass_description.resolve_attachments(&all_resolve_references[range]); }
            if let Some(index) = subpass.depth { subpass_description = subpass_description.depth_stencil_attachment(&all_output_references[index]); }
            subpass_description
        }).collect();
        //dependencies
        let mut subpass_dependencies = vec![];
        //external dependencies
        for attachment in color_attachments
        {
            match attachment
            {
                RenderPassColorAttachment::Swapchain(_) =>
                {
                    subpass_dependencies.push(vk::SubpassDependency::default()
                        .src_subpass(vk::SUBPASS_EXTERNAL)
                        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_subpass(0)
                        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                        .dependency_flags(vk::DependencyFlags::BY_REGION));
                },
                RenderPassColorAttachment::Image { store: AttachmentStore::Store, .. } =>
                {
                    subpass_dependencies.push(vk::SubpassDependency::default()
                        .src_subpass(subpasses.len() as u32 - 1)
                        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                        .dst_subpass(vk::SUBPASS_EXTERNAL)
                        .dst_stage_mask(vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER)
                        .dst_access_mask(vk::AccessFlags::SHADER_READ)
                        .dependency_flags(vk::DependencyFlags::BY_REGION));
                },
                _ =>{}
            }
        }
        if let Some(RenderPassDepthAttachment { store: AttachmentStore::Store, .. }) = depth_attachment
        {
            subpass_dependencies.push(vk::SubpassDependency::default()
                .src_subpass(subpasses.len() as u32 - 1)
                .src_stage_mask(vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
                .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .dst_stage_mask(vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .dependency_flags(vk::DependencyFlags::BY_REGION));
        }
        //internal dependencies
        for (input_index, input) in all_input_references.iter().enumerate()
        {
            for (output_index, output) in all_output_references.iter().enumerate()
            {
                if input.attachment == output.attachment
                {
                	let depth = Some(input.attachment) == depth_attachment_index;
                    subpass_dependencies.push(vk::SubpassDependency::default()
                        .src_subpass(all_output_subpass_indices[output_index])
                        .src_stage_mask(if depth { vk::PipelineStageFlags::LATE_FRAGMENT_TESTS } else { vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT })
                        .src_access_mask(if depth { vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE } else { vk::AccessFlags::COLOR_ATTACHMENT_WRITE })
                        .dst_subpass(all_input_subpass_indices[input_index])
                        .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
                        .dst_access_mask(vk::AccessFlags::INPUT_ATTACHMENT_READ)
                        .dependency_flags(vk::DependencyFlags::BY_REGION));
                }
            }
        }
        //render pass
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&subpass_dependencies);
        let render_pass = 'rp:
        {
            #[cfg(feature = "multiview")]
            if let Some(Multiview { count, spatial_coherency }) = multiview
            {
                let full_mask = (1 << count) - 1;
                let view_masks = vec![full_mask; subpasses.len()];
                let render_pass_multiview_create_info = vk::RenderPassMultiviewCreateInfoKHR::default()
                    .view_masks(&view_masks);
                let mut render_pass_multiview_create_info =
                    if spatial_coherency { render_pass_multiview_create_info.correlation_masks(std::slice::from_ref(&full_mask)) }
                    else { render_pass_multiview_create_info };
                let render_pass_info = render_pass_info.push_next(&mut render_pass_multiview_create_info);
                #[allow(unused_parens)] break 'rp (unsafe { self.0.logical_device.create_render_pass(&render_pass_info, None) });
            }
            #[allow(unused_parens)] break 'rp (unsafe { self.0.logical_device.create_render_pass(&render_pass_info, None) });
        }.unwrap();
        RenderPass { device: self.0.clone(), render_pass, clear_values: Box::from(clear_colors) }
    }

    pub fn new_framebuffer(&self, render_pass: &RenderPass, attachments: &[FramebufferAttachment]) -> Framebuffer
    {
        if DEBUG_MODE && attachments.len() == 0 { panic!("Device::new_framebuffer: At least 1 attachment is required."); }
        let (width, height) = attachments[0].dimensions();
        let mut image_views = Vec::new();
        let mut attachments_vec = Vec::with_capacity(attachments.len());
        for attachment in attachments.iter()
        {
        	if DEBUG_MODE && attachment.dimensions() != (width, height) { panic!("Swapchain::new_framebuffers: Inconsistent dimensions."); }
        	match attachment
        	{
        		FramebufferAttachment::Swapchain(SwapchainImage { image_view, .. }) => attachments_vec.push(**image_view),
        		FramebufferAttachment::Image { image, layer } =>
        		{
        			if let ImageUsage::Texture { .. } = image.image_usage { panic!("Swapchain::new_framebuffers: Texture cannot be used as attachment."); }
                    if let Some(layer) = layer
                    {
                        let image_view_create_info = vk::ImageViewCreateInfo::default()
                            .image(image.image)
                            .view_type(image.image_type.view_type())
                            .format(image.image_type.channel.vk_format())
                            .subresource_range(vk::ImageSubresourceRange
                            {
                                aspect_mask: if image.image_usage.depth() { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::COLOR },
                                base_mip_level: 0,
                                level_count: image.image_usage.mip_levels(image.image_type),
                                base_array_layer: *layer,
                                layer_count: 1
                            });
                        let image_view = unsafe { self.0.logical_device.create_image_view(&image_view_create_info, None) }.unwrap();
                        image_views.push(image_view);
                        attachments_vec.push(image_view);
                    } else { attachments_vec.push(image.image_view); }
        		}
        	}
        }
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass.render_pass)
            .attachments(&attachments_vec)
            .width(width)
            .height(height)
            .layers(1);
        let framebuffer = unsafe { self.0.logical_device.create_framebuffer(&framebuffer_info, None) }.unwrap();
        Framebuffer { device: self.0.clone(), image_views, framebuffer, size: (width, height) }
    }
}

#[derive(Clone, Copy)]
pub enum ColorAttachmentLoad
{
    Load,
    Clear { color: [f32; 4] },
    DontCare
}

impl ColorAttachmentLoad
{
    const fn vk_attachment_load_op(&self) -> vk::AttachmentLoadOp
    {
        match self
        {
            ColorAttachmentLoad::Load => vk::AttachmentLoadOp::LOAD,
            ColorAttachmentLoad::Clear { .. } => vk::AttachmentLoadOp::CLEAR,
            ColorAttachmentLoad::DontCare => vk::AttachmentLoadOp::DONT_CARE
        }
    }

    const fn vk_clear_value(&self) -> vk::ClearValue
    {
        match self
        {
            ColorAttachmentLoad::Clear { color } => vk::ClearValue { color: vk::ClearColorValue { float32: *color } },
            _ => vk::ClearValue { color: vk::ClearColorValue { float32: [0.0; 4] } }
        }
    }
}

#[derive(Clone, Copy)]
pub enum SwapchainLoad
{
    Clear { color: [f32; 4] },
    DontCare
}

impl SwapchainLoad
{
    const fn vk_attachment_load_op(&self) -> vk::AttachmentLoadOp
    {
        match self
        {
            SwapchainLoad::Clear { .. } => vk::AttachmentLoadOp::CLEAR,
            SwapchainLoad::DontCare => vk::AttachmentLoadOp::DONT_CARE
        }
    }

    const fn vk_clear_value(&self) -> vk::ClearValue
    {
        match self
        {
            SwapchainLoad::Clear { color } => vk::ClearValue { color: vk::ClearColorValue { float32: *color } },
            SwapchainLoad::DontCare => vk::ClearValue { color: vk::ClearColorValue { float32: [0.0; 4] } }
        }
    }
}

#[derive(Clone, Copy)]
pub enum DepthAttachmentLoad
{
    Load,
    Clear { depth: f32 },
    DontCare
}

impl DepthAttachmentLoad
{
    const fn vk_attachment_load_op(&self) -> vk::AttachmentLoadOp
    {
        match self
        {
            DepthAttachmentLoad::Load => vk::AttachmentLoadOp::LOAD,
            DepthAttachmentLoad::Clear { .. } => vk::AttachmentLoadOp::CLEAR,
            DepthAttachmentLoad::DontCare => vk::AttachmentLoadOp::DONT_CARE
        }
    }

    const fn vk_clear_value(&self) -> vk::ClearValue
    {
        match self
        {
            DepthAttachmentLoad::Clear { depth } => vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: *depth, stencil: 0 } },
            _ => vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } }
        }
    }
}

#[derive(Clone, Copy)]
pub enum AttachmentStore
{
    Store,
    DontCare
}

impl AttachmentStore
{
    const fn vk_attachment_store_op(&self) -> vk::AttachmentStoreOp
    {
        match self
        {
            AttachmentStore::Store => vk::AttachmentStoreOp::STORE,
            AttachmentStore::DontCare => vk::AttachmentStoreOp::DONT_CARE
        }
    }
}

#[derive(Clone, Copy)]
pub enum RenderPassColorAttachment
{
    Swapchain(SwapchainLoad),
    Image
    {
        image_channel_type: ImageChannelType,
        samples: Msaa,
        load: ColorAttachmentLoad,
        store: AttachmentStore,
        initial_layout: ImageLayout,
        final_layout: ImageLayout
    }
}

#[derive(Clone, Copy)]
pub struct RenderPassDepthAttachment
{
    pub image_channel_type: ImageChannelType,
    pub samples: Msaa,
    pub load: DepthAttachmentLoad,
    pub store: AttachmentStore,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout
}

#[derive(Clone, Copy)]
pub struct InputAttachment
{
    pub attachment_index: u32,
    pub fragment_input_attachment_index: u32
}

#[derive(Clone, Copy)]
pub struct OutputAttachment
{
    pub attachment_index: u32,
    pub fragment_out_location: u32
}

#[derive(Clone, Copy)]
pub enum ResolveAttachment
{
	Index(u32),
	Unused
}

#[derive(Clone, Copy)]
pub struct Subpass<'a>
{
    pub input_attachments: &'a [InputAttachment],
    pub output_attachments: &'a [OutputAttachment],
    pub resolve_attachments: Option<&'a [ResolveAttachment]>,
    pub depth_attachment: bool
}

#[cfg(feature = "multiview")]
#[derive(Clone, Copy)]
pub struct Multiview
{
    pub count: u32,
    pub spatial_coherency: bool
}

#[derive(Clone, Copy)]
pub struct RenderPassInfo<'a>
{
    pub color_attachments: &'a [RenderPassColorAttachment],
    pub depth_attachment: Option<RenderPassDepthAttachment>,
    pub subpasses: &'a [Subpass<'a>],
    #[cfg(feature = "multiview")]
    pub multiview: Option<Multiview>
}

#[derive(Clone, Copy)]
pub enum FramebufferAttachment<'a>
{
    Swapchain(SwapchainImage<'a>),
    Image { image: &'a Image, layer: Option<u32> }
}

impl<'a> FramebufferAttachment<'a>
{
    pub fn image(image: &'a Image) -> Self
    {
        Self::Image { image, layer: None }
    }
    
	const fn dimensions(&self) -> (u32, u32)
	{
		match self
		{
			FramebufferAttachment::Swapchain(SwapchainImage { width, height, .. }) => (*width, *height),
			FramebufferAttachment::Image { image, .. } => (image.image_type.width, image.image_type.height)
		}
	}
}
