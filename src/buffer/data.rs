use super::*;
#[cfg(feature = "math")]
use gru_misc::math::{Vec2, Vec3, Vec4};

impl IndexType for u16
{
    const FORMAT: vk::IndexType = vk::IndexType::UINT16;
}

impl IndexType for u32
{
    const FORMAT: vk::IndexType = vk::IndexType::UINT32;
}

impl AttributeType
{
    pub(crate) const fn vk_format(&self) -> vk::Format
    {
        match self
        {
            AttributeType::F1 => vk::Format::R32_SFLOAT,
            AttributeType::F2 => vk::Format::R32G32_SFLOAT,
            AttributeType::F3 => vk::Format::R32G32B32_SFLOAT,
            AttributeType::F4 => vk::Format::R32G32B32A32_SFLOAT,
            AttributeType::I1 => vk::Format::R32_SINT,
            AttributeType::I2 => vk::Format::R32G32_SINT,
            AttributeType::I3 => vk::Format::R32G32B32_SINT,
            AttributeType::I4 => vk::Format::R32G32B32A32_SINT,
            AttributeType::U1 => vk::Format::R32_UINT,
            AttributeType::U2 => vk::Format::R32G32_UINT,
            AttributeType::U3 => vk::Format::R32G32B32_UINT,
            AttributeType::U4 => vk::Format::R32G32B32A32_UINT
        }
    }

    pub(crate) const fn size_in_bytes(&self) -> u32
    {
        match self
        {
            AttributeType::F1 | AttributeType::I1 | AttributeType::U1 => 4,
            AttributeType::F2 | AttributeType::I2 | AttributeType::U2 => 8,
            AttributeType::F3 | AttributeType::I3 | AttributeType::U3 => 12,
            AttributeType::F4 | AttributeType::I4 | AttributeType::U4 => 16,
        }
    }
}

//     #####     F1 - F4

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct F1(f32);

impl From<f32> for F1
{
    #[inline]
    fn from(x: f32) -> Self { F1(x) }
}

impl Into<f32> for F1
{
    #[inline]
    fn into(self) -> f32 { self.0 }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct F2(f32, f32);

impl From<(f32, f32)> for F2
{
    #[inline]
    fn from((x, y): (f32, f32)) -> Self { F2(x, y) }
}

impl Into<(f32, f32)> for F2
{
    #[inline]
    fn into(self) -> (f32, f32) { (self.0, self.1) }
}

#[cfg(feature = "math")]
impl From<Vec2> for F2
{
    #[inline]
    fn from(Vec2(x, y): Vec2) -> Self { F2(x, y) }
}

#[cfg(feature = "math")]
impl Into<Vec2> for F2
{
    #[inline]
    fn into(self) -> Vec2 { Vec2(self.0, self.1) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct F3(f32, f32, f32);

impl From<(f32, f32, f32)> for F3
{
    #[inline]
    fn from((x, y, z): (f32, f32, f32)) -> Self { F3(x, y, z) }
}

impl Into<(f32, f32, f32)> for F3
{
    #[inline]
    fn into(self) -> (f32, f32, f32) { (self.0, self.1, self.2) }
}

#[cfg(feature = "math")]
impl From<Vec3> for F3
{
    #[inline]
    fn from(Vec3(x, y, z): Vec3) -> Self { F3(x, y, z) }
}

#[cfg(feature = "math")]
impl Into<Vec3> for F3
{
    #[inline]
    fn into(self) -> Vec3 { Vec3(self.0, self.1, self.2) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct F4(f32, f32, f32, f32);

impl From<(f32, f32, f32, f32)> for F4
{
    #[inline]
    fn from((x, y, z, w): (f32, f32, f32, f32)) -> Self { F4(x, y, z, w) }
}

impl Into<(f32, f32, f32, f32)> for F4
{
    #[inline]
    fn into(self) -> (f32, f32, f32, f32) { (self.0, self.1, self.2, self.3) }
}

#[cfg(feature = "math")]
impl From<Vec4> for F4
{
    #[inline]
    fn from(Vec4(x, y, z, w): Vec4) -> Self { F4(x, y, z, w) }
}

#[cfg(feature = "math")]
impl Into<Vec4> for F4
{
    #[inline]
    fn into(self) -> Vec4 { Vec4(self.0, self.1, self.2, self.3) }
}

//     #####     I1 - I4

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct I1(i32);

impl From<i32> for I1
{
    #[inline]
    fn from(x: i32) -> Self { I1(x) }
}

impl Into<i32> for I1
{
    #[inline]
    fn into(self) -> i32 { self.0 }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct I2(i32, i32);

impl From<(i32, i32)> for I2
{
    #[inline]
    fn from((x, y): (i32, i32)) -> Self { I2(x, y) }
}

impl Into<(i32, i32)> for I2
{
    #[inline]
    fn into(self) -> (i32, i32) { (self.0, self.1) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct I3(i32, i32, i32);

impl From<(i32, i32, i32)> for I3
{
    #[inline]
    fn from((x, y, z): (i32, i32, i32)) -> Self { I3(x, y, z) }
}

impl Into<(i32, i32, i32)> for I3
{
    #[inline]
    fn into(self) -> (i32, i32, i32) { (self.0, self.1, self.2) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct I4(i32, i32, i32, i32);

impl From<(i32, i32, i32, i32)> for I4
{
    #[inline]
    fn from((x, y, z, w): (i32, i32, i32, i32)) -> Self { I4(x, y, z, w) }
}

impl Into<(i32, i32, i32, i32)> for I4
{
    #[inline]
    fn into(self) -> (i32, i32, i32, i32) { (self.0, self.1, self.2, self.3) }
}

//     #####     U1 - U4

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct U1(u32);

impl From<u32> for U1
{
    #[inline]
    fn from(x: u32) -> Self { U1(x) }
}

impl Into<u32> for U1
{
    #[inline]
    fn into(self) -> u32 { self.0 }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct U2(u32, u32);

impl From<(u32, u32)> for U2
{
    #[inline]
    fn from((x, y): (u32, u32)) -> Self { U2(x, y) }
}

impl Into<(u32, u32)> for U2
{
    #[inline]
    fn into(self) -> (u32, u32) { (self.0, self.1) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct U3(u32, u32, u32);

impl From<(u32, u32, u32)> for U3
{
    #[inline]
    fn from((x, y, z): (u32, u32, u32)) -> Self { U3(x, y, z) }
}

impl Into<(u32, u32, u32)> for U3
{
    #[inline]
    fn into(self) -> (u32, u32, u32) { (self.0, self.1, self.2) }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(gru_vulkan_derive::AttributeType)]
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct U4(u32, u32, u32, u32);

impl From<(u32, u32, u32, u32)> for U4
{
    #[inline]
    fn from((x, y, z, w): (u32, u32, u32, u32)) -> Self { U4(x, y, z, w) }
}

impl Into<(u32, u32, u32, u32)> for U4
{
    #[inline]
    fn into(self) -> (u32, u32, u32, u32) { (self.0, self.1, self.2, self.3) }
}
