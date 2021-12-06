use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::{ComputePipeline, PipelineBindPoint};

use vulkano::sync;
use vulkano::sync::GpuFuture;
use vulkano::Version;

use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::ImageDimensions;
use vulkano::image::StorageImage;

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    let decoder = png::Decoder::new(
        File::open(r"C:\Users\shashi\Documents\SchoolVarshith\imgpu\gpu_image\src\bonjour.png")
            .unwrap(),
    );
    let mut reader = decoder.read_info().unwrap();
    let mut buf2: Vec<u8> = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf2).unwrap();

    let mut buf2_rgba = Vec::new();
    let mut index = 1;
    // Turn buf from RGB to RGBA
    for (_, i) in buf2.iter().enumerate() {
        if index == 3 {
            buf2_rgba.push(*i);
            buf2_rgba.push(255);
            index = 1;
        } else {
            buf2_rgba.push(*i);
            index += 1;
        }
    }
    let bytes = &buf2_rgba[..];

    /* #region Vulkan */
    /*------------------------------------------------------------
    :::      .::....    ::: :::      :::  .    :::.   :::.    :::.
    ';;,   ,;;;' ;;     ;;; ;;;      ;;; .;;,. ;;`;;  `;;;;,  `;;;
     \[[  .[[/  [['     [[[ [[[      [[[[[/'  ,[[ '[[,  [[[[[. '[[
      Y$c.$$"   $$      $$$ $$'     _$$$$,   c$$$cc$$$c $$$ "Y$c$$
       Y88P     88    .d888 88oo,.__"888"88o, 888   888,888    Y88
        MP       "YmmMMMM"" """"YUMMM MMM "MMP"YMM   ""` MMM     YM
    -------------------------------------------------------------*/

    let instance = Instance::new(None, Version::V1_1, &InstanceExtensions::none(), None).unwrap();

    let device_extensions = DeviceExtensions {
        khr_storage_buffer_storage_class: true,
        ..DeviceExtensions::none()
    };
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_compute())
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .unwrap();

    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type
    );

    let (device, mut queues) = Device::new(
        physical_device,
        &Features::none(),
        &physical_device
            .required_extensions()
            .union(&device_extensions),
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();

    let queue = queues.next().unwrap();

    let pipeline = Arc::new({
        mod cs {
            vulkano_shaders::shader! {
                ty: "compute",
                src: "
                #version 450

                layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
                
                layout(set = 0, binding = 0, rgba8) uniform image2D img;
                
                void main() {
                    vec4 data = imageLoad(img, ivec2(gl_GlobalInvocationID.xy));
                    float avg = (data[0] + data[1] + data[2]) / 3.0;  
                    vec4 to_write = vec4(1.0 - data[0], 1.0 - data[1], 1.0 - data[2], 1.0);
                    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
                }
                "
            }
        }
        let shader = cs::Shader::load(device.clone()).unwrap();
        ComputePipeline::new(
            device.clone(),
            &shader.main_entry_point(),
            &(),
            None,
            |_| {},
        )
        .unwrap()
    });

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

    let layout = pipeline.layout().descriptor_set_layouts().get(0).unwrap();
    let mut set_builder = PersistentDescriptorSet::start(layout.clone());
    set_builder
        .add_image(ImageView::new(image.clone()).unwrap())
        .unwrap();

    let set = set_builder.build().unwrap();

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::SimultaneousUse,
    )
    .unwrap();

    builder
        .bind_pipeline_compute(pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            pipeline.layout().clone(),
            0,
            set,
        )
        .copy_buffer_to_image(image_buf.clone(), image.clone())
        .unwrap()
        .dispatch([info.width / 8, info.height / 8, 1])
        .unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone())
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    /* #endregion */

    let path =
        Path::new(r"C:\Users\shashi\Documents\SchoolVarshith\img_gpu\gpu_image\src\grayscale.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let data = buf.read().unwrap();
    let mut encoder = png::Encoder::new(w, info.width, info.height);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    let u8data = &data[..];
    writer.write_image_data(u8data).unwrap();
}
