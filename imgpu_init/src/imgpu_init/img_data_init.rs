use std::fs::File;

pub fn decode_image(path: &str) -> (png::OutputInfo, Vec<u8>) {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buffer: Vec<u8> = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).unwrap();

    (info, buffer)
}
