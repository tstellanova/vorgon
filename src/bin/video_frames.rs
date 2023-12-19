use ffmpeg_next as ffmpeg;

use ffmpeg::format::input as ffmpeg_input;
use ffmpeg::format::{Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::env;

use std::path::Path;
use image::{ImageBuffer};
use image::buffer::ConvertBuffer;
use regex::Regex;
use vorgon::{crop_rgb_to_percent};


fn main() -> Result<(), ffmpeg::Error> {
  ffmpeg::init().unwrap();
  let filename = env::args().nth(1).expect("need video filename");
  let start_frame = env::args().nth(2).expect("no start frame").parse::<usize>().unwrap();
  let end_frame = env::args().nth(3).expect("no end frame").parse::<usize>().unwrap();

  let regx = Regex::new(r"(\w+)\-(\d+)\.").unwrap();
  let hay = filename.clone();
  let finds = regx.captures(&hay).unwrap();
  let video_file_prefix = &finds[1];
  let video_id = &finds[2];
  println!("prefix: {:?} video_id: {:?}", video_file_prefix, video_id);

  let base_path_str= format!("./{}-{}-frames-rgb-{}",video_id,video_file_prefix,start_frame);
  let base_path = Path::new(&base_path_str);
  println!("output frames to: {:?}", base_path);
  std::fs::create_dir_all(base_path).expect("can't create output path");

  if let Ok(mut ictx) = ffmpeg_input(&filename) {
    let input = ictx
      .streams()
      .best(Type::Video)
      .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    let mut scaler = Context::get(
      decoder.format(),
      decoder.width(),
      decoder.height(),
      Pixel::RGB24,
      decoder.width(),
      decoder.height(),
      Flags::BILINEAR,
    )?;


    let mut receive_and_process_decoded_frames =
      |decoder: &mut ffmpeg::decoder::Video, frame_idx: usize| -> Result<(), ffmpeg::Error> {
        let mut decoded = Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
          if (frame_idx >= start_frame) && (frame_idx <= end_frame) {
            let mut rgb_frame = Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;
            process_frame(&rgb_frame, &base_path, &video_id, frame_idx).unwrap();
          }
        }
        Ok(())
      };

    // TODO this is a heuristic guess at where we'll find a keyframe prior to region of interest
    let min_packet_frame = start_frame-120;
    let max_packet_frame = end_frame+30;

    let mut packet_count = 0;
    for (stream, packet) in ictx.packets() {
      if stream.index() == video_stream_index {
        // println!("packet {}", packet_count);
        //prefilter of 60 frames avoids sending lots of extraneous stuff to decoder
        if packet_count == 0 ||
          ((packet_count > min_packet_frame) && (packet_count < max_packet_frame  )) {
          decoder.send_packet(&packet)?;
          receive_and_process_decoded_frames(&mut decoder, packet_count)?;
        }
        packet_count += 1;
        if packet_count > end_frame {
          break;
        }
      }
    }
    println!("final packet_count: {} start: {} end: {}",
             packet_count, start_frame, end_frame);
    decoder.send_eof()?;
    receive_and_process_decoded_frames(&mut decoder, packet_count)?;
  }

  Ok(())
}

// 1670019436 12:50 --> frame (12*60 + 50) * 30 = 23100
// 1670019436 12:30 --> frame (12*60 + 30) * 30 = 22500
// 1670019436 03:48 --> frame (3*603 + 48) * 30 = 6840

fn process_frame(frame: &Video, path: &Path, file_id: &str, index: usize) -> std::result::Result<(), std::io::Error> {
  // let img_bu: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
  let img_buf: ImageBuffer<image::Rgb<u8>, &[u8]> =
    image::ImageBuffer::from_raw(
    frame.width(), frame.height(), frame.data(0)).unwrap();

  // let gray_img: GrayImage = img_buf.convert();
  // let rgb_img: RgbImage = img_buf.convert();
  // our images have strong vignetting, so we crop out the edges
  let crop_img = crop_rgb_to_percent(&img_buf.convert(), 0.8);

  let file_name = format!("{}_frame_{}.jpg",file_id, index);
  let full_path = path.join(file_name.clone());
  println!("le_filename: {}", file_name);
  let _ = crop_img.save(full_path);

  Ok(())
}
