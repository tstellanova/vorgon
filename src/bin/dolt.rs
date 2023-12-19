
use std::{
    default::Default,
    env,
    fs,
    path::{Path, PathBuf}
};

use serde::Serialize;
use serde_json::{to_string_pretty};

use image::{DynamicImage, GrayImage, Luma};
use image::imageops::crop_imm;
use image_compare::Metric;
use imageproc::{
    corners::corners_fast12,
    filter::{
        // gaussian_blur_f32,
        // median_filter,
        filter3x3,
    },
};

use crate::*;




// pub fn process_directory() -> anyhow::Result<()> {
//     let mut tsms:i64 = 0;
//     timest(&mut tsms);
//
//     let base_path_str = env::args().nth(1).expect("need path with video frame images");
//     // let base_path_str = "/Users/toddstellanova/proj/rust-ffmpeg/1670019436-eo_wide_front_left-frames";
//     // let base_path_str = "/Users/toddstellanova/proj/rust-ffmpeg/1670019436-eo_wide_front_left-frames-6840";
//     // let base_path_str =
//     //   "/Users/toddstellanova/proj/rust-ffmpeg/1670019436-eo_wide_front_left-frames-23100";
//     let base_path = Path::new(&base_path_str);
//
//     println!("Loading images from: `{:?}` ", base_path);
//
//     // Read and sort the directory entries
//     let mut entries = fs::read_dir(base_path)
//       .unwrap_or_else(|err| {
//           eprintln!("Error reading directory: {}", err);
//           std::process::exit(1);
//       })
//       .filter_map(|entry| {
//           //println!("{:?}",entry);
//           if let Ok(res) = entry {
//               if let Some(ext) = res.path().as_path().extension() {
//                   if ext.eq_ignore_ascii_case("jpg") {
//                       return Some(res);
//                   }
//               }
//           }
//           None
//       })
//       .collect::<Vec<_>>();
//
//     entries.sort_by_key(|entry| entry.file_name());
//     let first_filename = entries.first().unwrap().path();
//     // println!("first_filename: {:?}", first_filename);
//     // let last_filename = entries.last()?.path();
//     // println!("last_filename: {:?}", last_filename);
//
//     let mut analyses: Vec<ImageQAttributes> = Vec::new();
//     let mut comparisons: Vec<ImgComparison> = Vec::new();
//
//     let first_img =
//       image::open(first_filename.clone())?.to_luma8();
//     println!("image 0 analysis");
//     let qattrs = analyze_image(&first_img);
//     analyses.push(qattrs);
//
//     let mut prior_img = first_img;
//     for i in 1..entries.len() {
//         println!("image {} analysis", i);
//         let entry_path:PathBuf = entries[i].path();
//         let raw_img =
//           image::open(entry_path.clone())?.to_luma8();
//         let cur_img = crop_to_80_percent(&raw_img);
//
//         // let cur_img = imageproc::noise::gaussian_noise(
//         //     &cur_img,6.0, 3.0, 5150);
//         // let cur_img = imageproc::noise::salt_and_pepper_noise(
//         //     &cur_img,0.2, 5150);
//
//         let qattrs = analyze_image(&cur_img);
//         analyses.push(qattrs);
//
//         println!("image {} compared to {}", i, i-1);
//         let (comp, color_map_opt) =
//           compare_images(&prior_img, &cur_img, true);
//         comparisons.push(comp);
//
//         // save if desired
//         if let Some(color_map) = color_map_opt {
//             let mut out_filename = entry_path.file_stem().unwrap().to_str().unwrap().to_string();
//             out_filename += "-SSIMAP.png";
//             let final_path = entry_path.with_file_name(out_filename);
//             color_map.save(final_path).unwrap();
//         }
//         prior_img = cur_img;
//     }
//
//     println!("analyses: {}", to_string_pretty(&analyses).unwrap());
//     println!("comparisons: {}",to_string_pretty(&comparisons).unwrap());
//
//     Ok(())
//     /*
//     let filename1 = "1670019436_frame_22814.ppm";
//     let filename2 = "1670019436_frame_22815.ppm";
//     let filename3 = "1670019436_frame_22816.ppm";
//
//     println!("{} >> open & convert to grays ",  timest(&mut tsms));
//     let img1 = image::open(filename1)?.to_luma8();
//     let img2 = image::open(filename2)?.to_luma8();
//     let img3 = image::open(filename3)?.to_luma8();
//     println!("{} >> end convert to grays ",  timest(&mut tsms));
//
//     println!("{} >> save grays ",  timest(&mut tsms));
//     img1.save("out_gray1.png").unwrap();
//     img2.save("out_gray2.png").unwrap();
//     img3.save("out_gray3.png").unwrap();
//     println!("{} << saved grays ",  timest(&mut tsms));
//
//
//     println!("===== start: clean sequence =====");
//     let clean_seq = vec!(img1.clone(), img2.clone(), img3.clone());
//     analyze_and_compare_sequence(&clean_seq);
//
//     let blur_radius = 4;
//     println!("{} >> start blur radius {}",  timest(&mut tsms), blur_radius);
//     let blur2 = median_filter(&img2, blur_radius, blur_radius);
//     blur2.save("out_blur2.png").unwrap();
//     println!("{} << end blur ",  timest(&mut tsms));
//
//     println!("===== start: dirty sequence =====");
//     let dirty_seq = vec!(img1.clone(), blur2.clone(), img3.clone());
//     analyze_and_compare_sequence(&dirty_seq);
//      */
//
//     // println!("===== analyze: img1 =====");
//     // analyze_image(&img1);
//     // println!("===== analyze: img2 =====");
//     // analyze_image(&img2);
//     // println!("===== analyze: blur2 =====");
//     // analyze_image(&blur2);
//     // println!("===== analyze: img3 =====");
//     // analyze_image(&img3);
//
//     // println!("{} >> start sharpness ",  timest(&mut tsms));
//     // let sharp_raw = calculate_laplacian_variance(&img2);
//     // let sharp_blur = calculate_laplacian_variance(&blur2);
//     // println!("raw sharpness: {} blurred sharpness: {}", sharp_raw, sharp_blur);
//     // println!("{} << end sharpness ",  timest(&mut tsms));
//     //
//     // println!("{} >> start corners ",  timest(&mut tsms));
//     // println!("img2 corners: {}",count_corners(&img2));
//     // println!("blur2 corners: {}",count_corners(&blur2));
//     // println!("{} << end corners ",  timest(&mut tsms));
//
//     // println!("{} >> start contrast ",  timest(&mut tsms));
//     // let (mean1, contrast1) = mean_intensity_and_contrast(&img1);
//     // let (mean2, contrast2) = mean_intensity_and_contrast(&img2);
//     // let (mean3, contrast3) = mean_intensity_and_contrast(&img2);
//     // println!("mean1 {} contrast1: {} ", mean1,  contrast1);
//     // println!("mean2 {} contrast2: {} ", mean2,  contrast2);
//     // println!("mean3 {} contrast3: {} ", mean3,  contrast3);
//     // println!("{} << end contrast ",  timest(&mut tsms));
//
//     // println!("{} >> start RMS:",timest(&mut tsms));
//     // let rms_error12 = imageproc::stats::root_mean_squared_error(&img1, &img2);
//     // let rms_error23 = imageproc::stats::root_mean_squared_error(&img2, &img3);
//     // println!("rms_error12: {}", rms_error12);
//     // println!("rms_error23: {}", rms_error23);
//     // println!("{} << end RMS ",  timest(&mut tsms));
//
//     // println!("{} >> start thresholds:",timest(&mut tsms));
//     // let thresh1 = imageproc::contrast::adaptive_threshold(&img1, 8);
//     // let thresh2 = imageproc::contrast::adaptive_threshold(&img2, 8);
//     // let thresh3 = imageproc::contrast::adaptive_threshold(&img3, 8);
//     // println!("{} << end thresholds", timest(&mut tsms));
//     //
//     // thresh1.save("out_thresh1.png").unwrap();
//     // thresh2.save("out_thresh2.png").unwrap();
//     // thresh3.save("out_thresh3.png").unwrap();
//     // println!("{} << saved thresholds",timest(&mut tsms));
//
//     // println!("{} >> start SSIM", timest(&mut tsms));
//     // let ssim12 = image_compare::gray_similarity_structure(
//     //     &image_compare::Algorithm::MSSIMSimple, &img1, &img2).unwrap();
//     // println!("SSIM12 score: {:?}",  ssim12.score);
//     // let ssim23 = image_compare::gray_similarity_structure(
//     //     &image_compare::Algorithm::MSSIMSimple, &img2, &img3).unwrap();
//     // println!("SSIM23 score: {}",  ssim23.score);
//     // let ssim_blur12 = image_compare::gray_similarity_structure(
//     //     &image_compare::Algorithm::MSSIMSimple, &img1, &blur2).unwrap();
//     // println!("SSIM blur score: {}", ssim_blur12.score);
//     // println!("{} << end SSIM", timest(&mut tsms));
//     //
//     // println!("{} >> start HSIM", timest(&mut tsms));
//     // let hsim12 = image_compare::gray_similarity_histogram(
//     //     Metric::Hellinger, &img1, &img2).unwrap();
//     // println!("HSIM12 score: {}",  hsim12);
//     // let hsim23 = image_compare::gray_similarity_histogram(
//     //     Metric::Hellinger, &img2, &img3).unwrap();
//     // println!("HSIM23 score: {}",  hsim23);
//     // println!("{} << end HSIM", timest(&mut tsms));
//
//
//     // println!("===== compare: 1 vs 2: =====");
//     // let (_comp, ssim12_grey) = compare_images(&img1, &img2);
//     // println!("===== compare: 2 vs 3: =====");
//     // let (_comp, ssim23_grey) = compare_images(&img2, &img3);
//     //
//     // println!("===== compare: 1 vs 2blur: =====");
//     // let (_comp, ssim_1_blur2_map) = compare_images(&img1, &blur2);
//     // println!("=====  compare: 2blur vs 3: =====");
//     // let (_comp, ssim_blur2_3_map) = compare_images(&blur2, &img3);
//
//     // ssim12_grey.save("out_ssim12.png").unwrap();
//     // ssim23_grey.save("out_ssim23.png").unwrap();
//     // ssim_1_blur2_map.save("out_ssim_1_blur2_map.png").unwrap();
//     // ssim_blur2_3_map.save("out_ssim_blur2_3_map.png").unwrap();
// }



fn analyze_local_images() -> anyhow::Result<()> {
    let mut tsms: i64 = 0;
    timest(&mut tsms);

    let filenames = vec!(
        // "1584484441_frame_100.ppm",
        // "1584484441_frame_5005.ppm",
        "1670019436_frame_22815.ppm",
        "1670019436_frame_22815_blur.ppm",
        "1670019436_frame_22815_overex.ppm",
        "1670019436_frame_22815_underex.ppm",
        "1670019436_frame_22815_overcont.ppm",
        "1670019436_frame_22815_undercont.ppm",
        );

    println!("{} >> open & convert to grays ",  timest(&mut tsms));
    let mut images = Vec::new();
    for i in 0..filenames.len() {
        let raw_img = image::open(filenames[i])?.to_luma8();
        let cropped_img = crop_to_80_percent(&raw_img);
        images.push(cropped_img);
    }
    println!("{} >> end convert to grays ",  timest(&mut tsms));

    println!("name,sharpness,mean_intensity,hist_spread,hist_flatness,corner_count");
    for i in 0..images.len() {
        let qattr = analyze_image(&images[i]);
        println!("{},{},{},{},{},{}",
                filenames[i],
                 qattr.sharpness,
                qattr.mean_intensity,
                qattr.hist_spread,
                 qattr.hist_flatness,
                 qattr.corner_count,
        );
        // println!("{} : {}",  filenames[i], to_string_pretty(&qattr)?);
    }

    Ok(())
}


fn main() -> anyhow::Result<()> {
    analyze_local_images()
}
