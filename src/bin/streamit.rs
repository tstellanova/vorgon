
use ffmpeg_next as ffmpeg;
use ffmpeg::format::input as ffmpeg_input;
use ffmpeg::format::{Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

use std::env;
// use std::path::Path;
use image::buffer::ConvertBuffer;

use image::{GrayImage, ImageBuffer};
use regex::Regex;
use vorgon::{crop_gray_to_percent, fast_analyze_image, MonoImageQAttributes};


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
  println!("frame,mean_intensity,hist_spread,f12_corners");

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

// 1670019436 12:50 --> frame (12*60 + 50) * 30 = 23100
// 1670019436 12:30 --> frame (12*60 + 30) * 30 = 22500
// 1670019436 03:48 --> frame (3*603 + 48) * 30 = 6840

fn process_frame(frame: &Video,  index: usize) -> std::result::Result<(), std::io::Error> {
  // for convenience we wrap the video data into an image::ImageBuffer
  let img_buf: ImageBuffer<image::Rgb<u8>, &[u8]> = image::ImageBuffer::from_raw(
    frame.width(), frame.height(), frame.data(0)).unwrap();

  // for image quality analysis we're mostly interested in grayscale
  let gray_img: GrayImage = img_buf.convert();

  // our images have strong vignetting, so we crop out the edges
  let crop_img = crop_gray_to_percent(&gray_img, 0.8);

  let qattr = fast_analyze_image(&crop_img);
  // if is_nominal(&qattr) {
    // Simple CSV output
    println!("{},{},{:0.6},{}",
             index,
             qattr.mean_intensity,
             qattr.hist_spread,
             qattr.corner_count_f12,
    );
  // }
  // else {
  //   let frame_mins = index / (30*60);
  //   let frame_secs = (index / 30) % 60;
  //   println!("invalid ({}) {:02}:{:02}", index, frame_mins, frame_secs);
  // }

  Ok(())
}

const INTENSITY_MEAN: f32 = 117.0;
const INTENSITY_STDDEV: f32  = 9.0;
const HSPREAD_MEAN: f32 = 0.5;
const HSPREAD_STDDEV: f32  = 0.09;
const F12_CORNERS_MEAN: f32 = 4000.0;
const F12_CORNERS_STDEV: f32 = 1000.0;


pub fn is_nominal(qattrs: &MonoImageQAttributes) -> bool
{
  let zscore_intense = zscore_within_stddev(
    INTENSITY_MEAN, INTENSITY_STDDEV, qattrs.mean_intensity as f32);
  let zscore_hspread = zscore_within_stddev(
    HSPREAD_MEAN, HSPREAD_STDDEV, qattrs.hist_spread as f32);
  let zscore_f12 = zscore_within_stddev(
    F12_CORNERS_MEAN, F12_CORNERS_STDEV, qattrs.corner_count_f12 as f32);

  if !zscore_intense { println!("bad intensity: {}", qattrs.mean_intensity)}
  if !zscore_hspread { println!("bad hspread: {}", qattrs.hist_spread)}
  if !zscore_f12 { println!("bad f12: {}", qattrs.corner_count_f12)}

  // println!("int {} hspread {} f12 {}", zscore_intense, zscore_hspread, zscore_f12);
  zscore_intense && zscore_hspread && zscore_f12
}

pub fn zscore_within_stddev(mean: f32, stddev: f32, val: f32) -> bool
{
  let zscore = (val - mean)/stddev;
  let nom = (zscore >= -2.0) && (zscore <= 2.0);
  // if !nom {
  //   println!("val: {} zscore: {}", val, zscore);
  // }
  nom
}
