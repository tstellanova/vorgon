
use ffmpeg_next as ffmpeg;
use ffmpeg::format::input as ffmpeg_input;
use ffmpeg::format::{Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

use std::env;
use std::path::Path;
// use std::path::Path;

use image::{ImageBuffer};
use regex::Regex;
use vorgon::{fast_analyze_image};


fn main() -> Result<(), ffmpeg::Error> {
  ffmpeg::init().unwrap();
  // TODO use clap instead for CLI args?
  let filename = env::args().nth(1).expect("need video filename");
  let start_frame = env::args().nth(2).expect("no start frame").parse::<usize>().unwrap();
  let end_frame = env::args().nth(3).expect("no end frame").parse::<usize>().unwrap();

  let regx = Regex::new(r"(\w+)\-(\d+)\.").unwrap();
  let hay = filename.clone();
  let finds = regx.captures(&hay).unwrap();
  let video_file_prefix = &finds[1];
  let video_id = &finds[2];

  println!("# prefix: {:?} video_id: {:?} start: {} end: {}",
           video_file_prefix, video_id, start_frame, end_frame);

  // CSV header
  // println!("frame,sharpness,mean_intensity,hist_spread,hist_flatness,corner_count");
  println!("frame,mean_intensity,hist_spread, dark_pct, bright_pct, f12_corners");


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


    // TODO this is a heuristic guess at where we'll find a keyframe prior to region of interest
    let min_packet_frame = start_frame-120;
    let max_packet_frame = end_frame+30;

    let mut packet_count = 0;
    for (stream, packet) in ictx.packets() {
      if stream.index() == video_stream_index {
        //prefilter of frames avoids sending lots of extraneous stuff to decoder
        if packet_count == 0 ||
          ((packet_count > min_packet_frame) && (packet_count < max_packet_frame  )) {
          decoder.send_packet(&packet)?;
          receive_and_process_decoded_frames(
            &mut decoder,
            &mut scaler,
            start_frame,
            end_frame,
            packet_count
          )?;
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
    receive_and_process_decoded_frames(
      &mut decoder, &mut scaler, start_frame, end_frame, packet_count)?;
  }

  Ok(())
}

fn receive_and_process_decoded_frames(
  decoder: &mut ffmpeg::decoder::Video,
  scaler: &mut Context,
  start_frame: usize,
  end_frame: usize,
  frame_idx: usize)
  -> Result<(), ffmpeg::Error>
{
  let mut decoded = Video::empty();
  while decoder.receive_frame(&mut decoded).is_ok() {
    if (frame_idx >= start_frame) && (frame_idx <= end_frame) {
      let mut rgb_frame = Video::empty();
      scaler.run(&decoded, &mut rgb_frame)?;
      process_frame(&rgb_frame,  frame_idx).unwrap();
    }
  }
  Ok(())
}




fn process_frame(frame: &Video,  index: usize) -> std::result::Result<(), std::io::Error> {
  // for convenience we wrap the video data into an image::ImageBuffer
  let img_buf: ImageBuffer<image::Rgb<u8>, &[u8]> = image::ImageBuffer::from_raw(
    frame.width(), frame.height(), frame.data(0)).unwrap();

  // println!("preproc: {}", index);
  let gray_img = vorgon::preprocess_rgb_to_gray(&img_buf);

  // TODO eliminate hardcoded paths
  let base_path = Path::new("/Users/toddstellanova/Desktop/runway-video/preproc/");
  let gray_file_name = format!("frame_{:06}_gray.jpg", index);
  let full_path = base_path.join(gray_file_name.clone());
  gray_img.save(full_path).unwrap();

  let rgb_file_name = format!("frame_{:06}_rgb.jpg", index);
  let full_path = base_path.join(rgb_file_name.clone());
  img_buf.save(full_path).unwrap();


  let qattr = fast_analyze_image(&gray_img);
  // if is_nominal(&qattr) {
  // Simple CSV output
  println!("{},{},{:0.4}, {:0.2},{:0.2}, {}",
           index,
           qattr.mean_intensity,
           qattr.hist_spread,

           qattr.dark_percent,
           qattr.bright_percent,

           qattr.corner_count_f12,
  );

  Ok(())
}


