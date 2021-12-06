use img_data_init::decode_image;
use vulkan_init::vulkan_init;

use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::ImageDimensions;
use vulkano::image::StorageImage;
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::{ComputePipeline, PipelineBindPoint};

pub mod img_data_init;
pub mod vulkan_init;

pub fn init(path: &str) {
    let (device, queue) = vulkan_init();
    let (info, buffer) = decode_image(path);

    let mut buf_rgba = Vec::new();
    let mut index = 1;
    // Turn buf from RGB to RGBA
    for (_, i) in buffer.iter().enumerate() {
        if index == 3 {
            buf_rgba.push(*i);
            buf_rgba.push(255);
            index = 1;
        } else {
            buf_rgba.push(*i);
            index += 1;
        }
    }
    let bytes = &buf_rgba[..];

    let image = StorageImage::new(
        device.clone(),
        ImageDimensions::Dim2d {
            width: info.width,
            height: info.height,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.family()),
    )
    .unwrap();

    let buf = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        (0..info.width * info.height * 4).map(|_| 0u8),
    )
    .expect("Failed to create buffer");

    let image_buf = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        bytes.iter().copied(),
    )
    .expect("Failed to create buffer");
}
