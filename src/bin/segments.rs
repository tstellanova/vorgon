use std::env;
use ffmpeg_next as ffmpeg;
use ffmpeg::format::input as ffmpeg_input;
use ffmpeg::format::{Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

// use std::env;
use serde::{Deserialize, Serialize};
use std::fs::{File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use image::buffer::ConvertBuffer;

use image::{GrayImage, ImageBuffer};
// use regex::Regex;
use vorgon::{crop_gray_to_percent, fast_analyze_image, MonoImageQAttributes};


fn process_video_segment(segment: &SegmentDecscriptor,
                         write_stream: &mut impl std::io::Write
)
{


  if let Ok(mut ictx) = ffmpeg_input(&segment.file_path) {
    let input = ictx
      .streams()
      .best(Type::Video)
      .ok_or(ffmpeg::Error::StreamNotFound).unwrap();
    let video_stream_index = input.index();

    let context_decoder =
      ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
    let mut decoder = context_decoder.decoder().video().unwrap();

    let mut scaler = Context::get(
      decoder.format(),
      decoder.width(),
      decoder.height(),
      Pixel::RGB24,
      decoder.width(),
      decoder.height(),
      Flags::BILINEAR,
    ).unwrap();

    // TODO this is a heuristic guess at where we'll find a keyframe prior to region of interest
    let min_packet_frame = if segment.start_frame > 120 { segment.start_frame-120 } else {0};
    let max_packet_frame = segment.end_frame+30;

    // CSV header
    let _ = write_stream.write_all(b"frame,i_mean,hspread,ncorners,nspikes,ndark,nbright");
    let _ = write_stream.write(b"\r\n");
    let _ = write_stream.flush();
    
    let mut packet_count:usize = 0;
    for (stream, packet) in ictx.packets() {
      if stream.index() == video_stream_index {
        //prefilter of frames avoids sending lots of extraneous stuff to decoder
        if packet_count == 0 ||
          ((packet_count > min_packet_frame as usize) && (packet_count < max_packet_frame as usize)) {
          decoder.send_packet(&packet).unwrap();
          receive_and_process_decoded_frames(
            &mut decoder,
            &mut scaler,
            segment.start_frame as usize,
            segment.end_frame as usize,
            packet_count,
            write_stream
          ).unwrap();
        }
        packet_count += 1;
        if packet_count > segment.end_frame as usize {
          break;
        }
      }
    }
    // println!("final packet_count: {} start: {} end: {}",
    //          packet_count, segment.start_frame, segment.end_frame);
    decoder.send_eof().unwrap();
    receive_and_process_decoded_frames(
      &mut decoder, &mut scaler,
      segment.start_frame as usize,
      segment.end_frame as usize,
      packet_count,
      write_stream
    ).unwrap();
  }
  else {
    eprintln!("Unable to open: {:?}",segment.file_path);
  }

}

fn receive_and_process_decoded_frames(
  decoder: &mut ffmpeg::decoder::Video,
  scaler: &mut Context,
  start_frame: usize,
  end_frame: usize,
  frame_idx: usize,
  write_stream: &mut impl std::io::Write )
  -> Result<(), ffmpeg::Error>
{
  let mut decoded = Video::empty();
  while decoder.receive_frame(&mut decoded).is_ok() {
    if (frame_idx >= start_frame) && (frame_idx <= end_frame) {
      let mut rgb_frame = Video::empty();
      scaler.run(&decoded, &mut rgb_frame)?;
      let summary = process_frame(&rgb_frame,  frame_idx).unwrap();
      write_stream.write_all(summary.as_bytes()).unwrap();
      write_stream.write(b"\r\n").unwrap();
    }
  }
  Ok(())
}

// 1670019436 12:50 --> frame (12*60 + 50) * 30 = 23100
// 1670019436 12:30 --> frame (12*60 + 30) * 30 = 22500
// 1670019436 03:48 --> frame (3*603 + 48) * 30 = 6840

fn process_frame(frame: &Video,  index: usize) -> std::result::Result<String, std::io::Error> {
  // for convenience we wrap the video data into an image::ImageBuffer
  let img_buf: ImageBuffer<image::Rgb<u8>, &[u8]> = image::ImageBuffer::from_raw(
    frame.width(), frame.height(), frame.data(0)).unwrap();

  // for image quality analysis we're mostly interested in grayscale
  let gray_img: GrayImage = img_buf.convert();

  // our images have strong vignetting, so we crop out the edges
  let crop_img = crop_gray_to_percent(&gray_img, 0.8);

  let qattr = fast_analyze_image(&crop_img);
  let frame_qstr = format!("{},{},{:0.6},{}, {},{},{}",
             index,
             qattr.mean_intensity,
             qattr.hist_spread,
             qattr.corner_count_f12,

              qattr.hist_spike_count,
              qattr.dark_pixel_count,
              qattr.bright_pixel_count,

  );

  Ok(frame_qstr)
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


#[derive(Serialize, Deserialize, Debug)]
struct Keypoint {
  frame: u32,
  x1: u32,
  y1: u32,
  x2: u32,
  y2: u32,
  // Add other fields as necessary
}

#[derive(Serialize, Deserialize, Debug)]
struct BoundingBox {
  frame: u32,
  tl_x: u32,
  tl_y: u32,
  br_x: u32,
  br_y: u32,
  // Add other fields as necessary
}


#[derive(Serialize, Deserialize, Debug)]
struct Approach {
  stream: String,
  start_frame: u32,
  end_frame: u32,
  icao: String,
  runway_designator: String,
  annotated_keypoints: Option<Keypoint>,
  annotated_bbox: Option<BoundingBox>,
  note: Option<String>,
  // Add other fields as necessary
}

#[derive(Serialize, Deserialize, Debug)]
struct Takeoff {
  // Define fields according to the JSON structure
}

#[derive(Serialize, Deserialize, Debug)]
struct FlightData {
  approaches: Option<Vec<Approach>>,
  takeoffs: Option<Vec<Takeoff>>,
}

#[derive(Default, Debug)]
struct SegmentDecscriptor {
  timestamp_str: String,
  file_path: PathBuf,
  start_frame: u32,
  end_frame: u32,
  validated_runway: bool,
}

fn get_segments_list(manifest_path: &Path) -> Vec<SegmentDecscriptor> {
  let mut segments:Vec<SegmentDecscriptor> = Vec::new();
  let mut manifest_file = File::open(&manifest_path).expect("approaches file not found");
  let mut contents = String::new();
  manifest_file.read_to_string(&mut contents).expect("couldn't read approaches file");

  let data: std::collections::HashMap<String, FlightData> =
    serde_json::from_str(&contents).expect("error while parsing approaches");

  // Example of how to access approaches and takeoffs
  for (timestamp, flight_data) in data {
    // println!("Timestamp: {}", timestamp);
    if let Some(approaches) = flight_data.approaches {
      // println!(">>> Approaches: ");
      for approach in approaches {
        let mut desc = SegmentDecscriptor::default();
        desc.timestamp_str = timestamp.clone();

        // println!("Approach: {}", serde_json::to_string_pretty(&approach).unwrap());
        if approach.note.is_some() {
          // failed to annotate
          desc.validated_runway = false;
          // println!("failed {},{},{},\"{}\"", approach.stream, approach.start_frame, approach.end_frame,
          //          approach.note.unwrap() );
          desc.start_frame = approach.start_frame;
          desc.end_frame = approach.end_frame;
        }
        else {
          // println!("valid {},{},{}", approach.stream, approach.start_frame, approach.end_frame);
          desc.start_frame = approach.start_frame;
          desc.end_frame = approach.end_frame;

          if approach.annotated_keypoints.is_some() ||
            approach.annotated_bbox.is_some() {
            desc.validated_runway = true;
          }
        }

        desc.file_path = manifest_path.with_file_name(approach.stream.clone() + ".mp4");
        println!("viz {} start {} end {} video: {:?}",
                 desc.validated_runway, desc.start_frame, desc.end_frame, desc.file_path);
        segments.push(desc);
      }
    }
  }

  segments
}

fn main() {
  let manifest_path_str = env::args().nth(1).expect("need manifest filename");
  ffmpeg::init().unwrap();

  let manifest_path = Path::new(&manifest_path_str);
  println!("manifest_path: {:?}",manifest_path);

  let segments = get_segments_list(&manifest_path);
  println!("nsegments: {}", segments.len());

  for seg in segments {
    if let Some(file_stem) = seg.file_path.file_stem()  {
      let outfile_namestr = format!("abrade_{}-{}-{}.csv",
                                    file_stem.to_str().unwrap() ,seg.start_frame, seg.end_frame);
      let out_path = manifest_path.with_file_name(outfile_namestr);
      println!("out_path: {:?}", out_path);
      let mut outfile = File::create(&out_path).unwrap();
      process_video_segment(&seg, &mut outfile);
      let _ = outfile.flush();
    }
  }

}


// // TODO use clap instead for CLI args?
// let filename = env::args().nth(1).expect("need video filename");
// let start_frame = env::args().nth(2).expect("no start frame").parse::<usize>().unwrap();
// let end_frame = env::args().nth(3).expect("no end frame").parse::<usize>().unwrap();
//
// let regx = Regex::new(r"(\w+)\-(\d+)\.").unwrap();
// let hay = filename.clone();
// let finds = regx.captures(&hay).unwrap();
// let video_file_prefix = &finds[1];
// let video_id = &finds[2];
//
// println!("# prefix: {:?} video_id: {:?} start: {} end: {}",
//          video_file_prefix, video_id, start_frame, end_frame);
