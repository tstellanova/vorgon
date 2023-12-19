

use image::{DynamicImage, imageops::crop_imm};

use image_compare::{
  Metric,
  prelude::*,
};

use imageproc::{
  corners::corners_fast12,
  filter::{
    // gaussian_blur_f32,
    // median_filter,
    filter3x3,
  },
};
// use imageproc::drawing::Canvas;

/// Describes the "inherent" quality of an image,
/// with no reference to another image.
#[derive(Debug)]
#[derive(Default)]
pub struct ImageQAttributes {
  pub width: u32,
  pub height: u32,
  pub sharpness: f32,
  pub mean_intensity: u8,
  pub hist_spread: f64,
  pub hist_flatness: f64,
  pub corner_count: u32,
}

/// Crop image to some percentage of its original dimensions:
/// attempts to preserve aspect ratio
// pub fn crop_gray_to_percent(raw_img: &GrayImage, percent: f32) -> GrayImage {
//   let (width, height) = raw_img.dimensions();
//
//   // Calculate N% of the dimensions
//   let new_width = (width as f32 * percent) as u32;
//   let new_height = (height as f32 * percent) as u32;
//   let left = (width - new_width) / 2;
//   let top = (height - new_height) / 2;
//
//   let cropped_img = crop_imm(raw_img, left, top, new_width, new_height).to_image();
//   cropped_img
//   // DynamicImage::from(cropped_img)
// }

pub fn crop_gray_to_percent(raw_img: &GrayImage, percent: f32) -> GrayImage
{
  let (width, height) = raw_img.dimensions();

  // Calculate N% of the dimensions
  let new_width = (width as f32 * percent) as u32;
  let new_height = (height as f32 * percent) as u32;
  let left = (width - new_width) / 2;
  let top = (height - new_height) / 2;

  crop_imm(raw_img, left, top, new_width, new_height).to_image()
}

pub fn crop_rgb_to_percent(raw_img: &RgbImage, percent: f32) -> RgbImage
{
  let (width, height) = raw_img.dimensions();

  // Calculate N% of the dimensions
  let new_width = (width as f32 * percent) as u32;
  let new_height = (height as f32 * percent) as u32;
  let left = (width - new_width) / 2;
  let top = (height - new_height) / 2;

  crop_imm(raw_img, left, top, new_width, new_height).to_image()
}


/// One way to measure sharpness
pub fn laplacian_variance(image: &GrayImage) -> (GrayImage, f32) {
  // Define the Laplacian kernel
  let laplacian_kernel: [f32; 9] = [0.0, -1.0, 0.0, -1.0, 4.0, -1.0, 0.0, -1.0, 0.0];

  // Apply the Laplacian filter
  let filtered_image = filter3x3::<Luma<u8>, f32, u8>(image, &laplacian_kernel);

  // Calculate the variance
  let mut total_intensity: u32 = 0;
  let mut total_sum_squares: u64 = 0;
  // let mut mean: f32 = 0.0;
  // let mut sum_of_squares: f32 = 0.0;
  let mut count = 0;

  for pixel in filtered_image.pixels() {
    let value = pixel.0[0] as u32;
    total_intensity += value;
    total_sum_squares += (value*value) as u64;
    count += 1;
  }

  if count == 0 {
    return (filtered_image, 0.0); // To handle an empty image case
  }

  let mean:f32 = (total_intensity as f32)/(count as f32);
  let sum_of_squares:f32 = ((total_sum_squares as f64) / (count as f64)) as f32;
  let score = sum_of_squares - (mean * mean);
  (filtered_image, score)
}

/// Returns (mean_intensity, histogram_spread, histogram_flatness)
pub fn hist_mean_spread_flatness(image: &GrayImage) -> (u8, f64, f64)
{
  let channel_hist = imageproc::stats::histogram(&image);
  let mut min_intensity = u8::MAX;
  let mut max_intensity = u8::MIN;
  let mut total_intensity: usize = 0;
  let mut cumulative_count: usize = 0;
  let mut first_quartile = 0;
  let mut third_quartile = 0;
  let mut hfm = 0.0; // Histogram Flatness Measure

  // TODO don't we already know the total number of pixels already from image size
  let total_pixels: usize = (image.width() * image.height()) as usize;

  if let Some(hist) = channel_hist.channels.first() {
    let measured_total_pixels = hist.iter().sum::<u32>() as usize;
    assert_eq!(total_pixels, measured_total_pixels);
    let total_pixels_f64 = total_pixels as f64;
    let first_quartile_count = total_pixels / 4;
    let third_quartile_count =  3 * total_pixels / 4;

    for i in 0..256 {
      let count = hist[i] as usize;
      if count > 0 {
        let intensity = i as u8;
        if intensity > max_intensity { max_intensity = intensity; }
        if intensity < min_intensity { min_intensity = intensity; }
        total_intensity += count * (intensity as usize);
        // flatness calculation
        let probability = count as f64 / total_pixels_f64;
        hfm -= probability * probability.log2(); // calculate entropy
        // spreading calculation
        cumulative_count += count;
        if (first_quartile == 0) && (cumulative_count >= first_quartile_count) {
          first_quartile = i;
        }
        if (third_quartile == 0) && (cumulative_count >= third_quartile_count) {
          third_quartile = i;
        }
      }
    }
  }

  let quartile_distance = third_quartile  - first_quartile ;
  let hist_spread = (quartile_distance as f64) / 255.0;
  let mean_intensity = (total_intensity / total_pixels) as u8;

  (mean_intensity, hist_spread, hfm)
}

// pub fn mean_intensity_and_contrast(image: &GrayImage) -> (u8, u8) {
//   let mut max_intensity: u8 = 0;
//   let mut min_intensity: u8 = 255;
//   let mut total_intensity: u32 = 0;
//
//   // Iterate over each pixel to find the max and min intensity values
//   for pixel in image.pixels() {
//     let intensity = pixel.0[0];
//     total_intensity += intensity as u32;
//     if intensity > max_intensity {
//       max_intensity = intensity;
//     }
//     if intensity < min_intensity {
//       min_intensity = intensity;
//     }
//   }
//   let avg_intensity = (total_intensity / image.pixels().count() as u32) as u8;
//   let contrast = max_intensity - min_intensity;
//   (avg_intensity, contrast)
// }

/// Measure the no-reference quality attributes of an image
pub fn analyze_image(img: &GrayImage)  -> ImageQAttributes {
  let mut durations: Vec<u32> = Vec::new();
  let mut tsms:i64 = 0;

  let mut qattrs = ImageQAttributes::default();
  qattrs.width = img.width();
  qattrs.height = img.height();

  timest(&mut tsms);
  // // println!("{} >> start sharpness ",  timest(&mut tsms));
  let (_laplace_img, sharpness ) = laplacian_variance(&img);
  qattrs.sharpness = sharpness;
  // // println!("{} << end sharpness ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start histo ",  timest(&mut tsms));
  let (mean, spread, flatness) = hist_mean_spread_flatness(&img);
  qattrs.mean_intensity = mean;
  qattrs.hist_spread = spread;
  qattrs.hist_flatness = flatness;
  // println!("{} << end histo ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start corners ",  timest(&mut tsms));
  qattrs.corner_count = count_corners(&img);
  // println!("{} << end corners ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  // println!("analyze durations {:?}", durations);

  qattrs
}

/// Count the number of FAST12 corners in an image
pub fn count_corners(img: &GrayImage) -> u32 {
  let all_corners = corners_fast12(&img, 32);
  all_corners.len() as u32
}

/// Represents a comparison between two images,
/// where one image provides a reference for comparison.
#[derive(Debug)]
#[derive(Default)]
pub struct ImgComparison {
  pub rms_error: f64,
  pub ssim_score: f64,
  pub hsim_score: f64,
}


/// Image comparisons
pub fn compare_images(img1: &GrayImage, img2: &GrayImage, gen_map: bool)
  -> (ImgComparison, Option<DynamicImage>)
{
  let mut durations: Vec<u32> = Vec::new();
  let mut tsms:i64 = 0;
  let mut comparison = ImgComparison::default();

  timest(&mut tsms);
  // println!("{} >> start RMS:",timest(&mut tsms));
  comparison.rms_error = imageproc::stats::root_mean_squared_error(img1, img2);
  // println!("{} << end RMS ",  timex(&mut tsms, &mut durations));
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start SSIM", timest(&mut tsms));
  let ssim = image_compare::gray_similarity_structure(
    &image_compare::Algorithm::MSSIMSimple, &img1, &img2).unwrap();
  // println!("{} << end SSIM", timex(&mut tsms, &mut durations));
  comparison.ssim_score = ssim.score;
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  //println!("{} >> start HSIM", timest(&mut tsms));
  comparison.hsim_score = image_compare::gray_similarity_histogram(
    // Metric::Hellinger, &img1, &img2).unwrap();
    Metric::Correlation, &img1, &img2).unwrap();
  // println!("{} << end HSIM", timex(&mut tsms, &mut durations));
  timex(&mut tsms, &mut durations);

  let ssim_color_map = if gen_map {
    timest(&mut tsms);
    // println!("{} >> start color_map", timest(&mut tsms));
    let color_map = ssim.image.to_color_map();
    // println!("{} << end color_map", timex(&mut tsms, &mut durations));
    timex(&mut tsms, &mut durations);
    Some(color_map)
  } else { None };

  // println!("{}",comparison);
  // println!("compare durations{:?}", durations);
  (comparison, ssim_color_map)
}



fn timest(last: &mut i64) -> i64 {
  let cur = chrono::Local::now().timestamp_millis();
  let delta = cur - *last;
  *last = cur;
  delta
}

fn timex(last: &mut i64, durations: &mut Vec<u32>) -> u32 {
  let duration = timest(last) as u32;
  durations.push(duration );
  duration
}

//fn histogram_spread(histogram: [u32; 256]) -> f32 {
//     let total_count: u32 = histogram.iter().sum();
//     let mut cumulative_sum = 0;
//     let mut first_quartile = 0;
//     let mut third_quartile = 0;
//
//     for (i, &count) in histogram.iter().enumerate() {
//         cumulative_sum += count;
//         if cumulative_sum >= total_count / 4 && first_quartile == 0 {
//             first_quartile = i;
//         }
//         if cumulative_sum >= 3 * total_count / 4 {
//             third_quartile = i;
//             break;
//         }
//     }
//
//     let quartile_distance = third_quartile as f32 - first_quartile as f32;
//     let range = 255.0; // For an 8-bit grayscale image
//
//     quartile_distance / range
// }