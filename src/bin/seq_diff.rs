

//! Process some frame pairs and examine similarity

use std::env;

use serde::{Deserialize, Serialize};
use std::fs::{File};
use std::io::{Read};
use std::path::{Path, PathBuf};

use image::{GrayImage};
use vorgon::{compare_images, fast_analyze_image};


fn process_frame(first: &GrayImage, second: &GrayImage) -> Result<(String, f64, f64), std::io::Error> {

  // let qattr1 = fast_analyze_image(&first);
  // let qattr2 = fast_analyze_image(&second);

  let (cmp, _) = compare_images(&first, &second, false);

  // write the CSV of frame analysis

  // let first_attr_str = format!("\
  // \r\nmean_intensity, hist_spread, corner_count_f12, dark_percent, bright_percent \
  // \r\n{},{:0.6},{}, {:0.2},{:0.2} \r\n",
  //                         qattr1.mean_intensity,
  //                         qattr1.hist_spread,
  //                         qattr1.corner_count_f12,
  //                         qattr1.dark_percent,
  //                         qattr1.bright_percent,
  // );
  // let second_attr_str = format!("\
  // {},{:0.6},{}, {:0.2},{:0.2} \r\n",
  //                         qattr2.mean_intensity,
  //                         qattr2.hist_spread,
  //                         qattr2.corner_count_f12,
  //                         qattr2.dark_percent,
  //                         qattr2.bright_percent,
  // );

  let cmp_str = format!("hsim {:+e}, ssim {:+e}",
                        cmp.hsim_score,
                        cmp.ssim_score,
  );

  // let res = first_attr_str + &second_attr_str + &cmp_str;
  let res = cmp_str;

  Ok( (res, cmp.hsim_score, cmp.ssim_score) )
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
  annotated_frame: u32,
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

          if let Some(keypt) = approach.annotated_keypoints {
            desc.validated_runway = true;
            desc.annotated_frame = keypt.frame;
          }
          else if let Some(bbox) = approach.annotated_bbox {
            desc.validated_runway = true;
            desc.annotated_frame = bbox.frame;
          }
        }

        desc.file_path = manifest_path.with_file_name(approach.stream.clone() + ".mp4");
        // println!("annotated? {} start: {} end: {} video: {:?}",
        //          desc.validated_runway, desc.start_frame, desc.end_frame, desc.file_path);
        segments.push(desc);

      }
    }
  }

  segments
}

fn compare_two(video_id: &str, frame_id: u32, one:&PathBuf, two: &PathBuf) -> Option<(f64, f64)> {
  // the image frame collected by the slow process is more accurate?
  if let Ok(first_image) = image::open(one.clone()) {
    // the image collected by the fast process is usually within 4 frames of the slow process
    if let Ok(second_image) = image::open(two.clone()) {
      let first_gray = first_image.into_luma8();
      let second_gray = second_image.into_luma8();
      if let Ok((cmp_str, hsim, ssim) ) = process_frame(&first_gray, &second_gray) {
        println!("\r\n=== video_id: {} frame_id: {} === {} ", video_id, frame_id, cmp_str);
        return Some((hsim, ssim) );
      }
    }
    else {
      println!("\r\n<<< missing second file path: {:?}", two);
    }
  }
  else {
    println!("\r\n<<< missing first file path: {:?}", one);
  }
  None
}

fn main() {
  let manifest_path_str = env::args().nth(1).expect("need manifest file path");

  let manifest_path = Path::new(&manifest_path_str);
  println!("manifest_path: {:?}",manifest_path);

  let segments = get_segments_list(&manifest_path);
  println!("nsegments: {}", segments.len());

  for seg in segments {
    if seg.validated_runway {
      if let Some(file_stem) = seg.file_path.file_stem() {
        let video_prefix: Vec<&str> = file_stem.to_str().unwrap().split("-").collect();
        let video_id = video_prefix[1];
        let slow_file_str = format!("{}_slow_{}.png",
                                    video_id, seg.annotated_frame);
        let fast_file_str = format!("{}_fast_{}.png",
                                    video_id, seg.annotated_frame);
        let delta_file_str = format!("{}_delta_{}.png",
                                     video_id, seg.annotated_frame);
        let parent_dir = manifest_path.parent().unwrap();
        // println!("parent_dir: {:?}", parent_dir);

        let target_dir = parent_dir.join("annotated_frames/");
        // println!("target_dir: {:?}", target_dir);

        let slow_file_path = target_dir.join(slow_file_str);
        let fast_file_path = target_dir.join(fast_file_str);
        let delta_file_path = target_dir.join(delta_file_str);

        if let Some((hsim0, ssim0)) = compare_two(
          &video_id, seg.annotated_frame, &slow_file_path, &slow_file_path) {
          if ssim0 < 1.0 || hsim0 < 1.0 {
            println!("<<< baseline SSIM: {} HSIM: {}", ssim0, hsim0);
          }
        }

        if let Some( (hsim1, ssim1) ) =  compare_two(
          &video_id, seg.annotated_frame, &slow_file_path, &fast_file_path) {
          if let Some((hsim2, ssim2)) = compare_two(
            &video_id, seg.annotated_frame, &slow_file_path, &delta_file_path) {
            if ssim1 > ssim2 {
              let ssim_diff = ssim1 - ssim2;
              println!("<<< Fast most similar SSIM by {:0.9}. HSIM {:0.8} vs {:0.8}",
                       ssim_diff, hsim1, hsim2
              );
            }
            else if ssim2 > ssim1 {
              let ssim_diff = ssim2 - ssim1;
              println!("<<< Delta most similar SSIM by {:0.9}. HSIM {:0.8} vs {:0.8}",
                       ssim_diff, hsim2, hsim1
              );
            }
            else {
              if hsim1 == hsim2 {
                println!("<<< Delta and Fast fully identical? SSIM {:+e}  HSIM {:+e} ",
                         ssim1, hsim1);
              }
              else {
                // in practice we don't ever seem to hit this
                println!("<<< Delta and Fast SSIM identical: {:+e}. HSIM {:+e} vs {:+e}",
                         ssim1, hsim2, hsim1);
              }
            }
          }
        }

      }
    }
  }

}
