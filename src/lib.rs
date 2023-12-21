

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
  // stats::ChannelHistogram,
};
use imageproc::corners::corners_fast9;
// use imageproc::drawing::Canvas;

/// Describes the "inherent" quality of a single-channel image
/// with no reference to another image.
#[derive(Debug)]
#[derive(Default)]
pub struct MonoImageQAttributes {
  pub width: u32,
  pub height: u32,
  /// Measurement of sharpness, maybe Laplacian variance or zero
  pub sharpness: f32,
  /// The mean intensity of all pixels
  pub mean_intensity: u8,
  /// Histogram spread
  pub hist_spread: f64,
  /// Histogram flatness
  pub hist_flatness: f64,
  /// Count of pixels below the "low" brightness threshold
  pub dark_pixel_count: u32,
  /// Count of pixels above the "high" brightness threshold
  pub bright_pixel_count: u32,
  /// Count of spikes in the histogram
  pub hist_spike_count: u32,
  /// FAST9 corners
  pub corner_count_f12: u32,
  /// FAST12 corners
  pub corner_count_f9: u32,
  // Raw histogram
  // pub raw_histogram: [u32; 256],
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
pub fn hist_mean_spread_flatness(image: &GrayImage, qattrs: &mut MonoImageQAttributes)
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

  qattrs.mean_intensity = mean_intensity;
  qattrs.hist_spread = hist_spread;
  qattrs.hist_flatness = hfm;
  // qattrs.raw_histogram = channel_hist;

}


/// Performs histogram analysis
pub fn fast_histogram_analysis(image: &GrayImage, qattrs: &mut MonoImageQAttributes)
{
  let channel_hist = imageproc::stats::histogram(&image);
  let mut min_intensity = u8::MAX;
  let mut max_intensity = u8::MIN;
  let mut total_intensity: usize = 0;
  let mut cumulative_count: usize = 0;
  let mut first_quartile = 0;
  let mut third_quartile = 0;

  let total_pixels: usize = (image.width() * image.height()) as usize;

  let spike_threshold = (total_pixels as f32 * 0.1 ) as i32;
  let mut prev_count: i32 = 0;

  if let Some(hist) = channel_hist.channels.first() {
    let first_quartile_count = total_pixels / 4;
    let third_quartile_count =  3 * total_pixels / 4;

    for i in 0..256 {
      let count = hist[i] as i32;
      if count > 0 {
        let intensity = i as u8;
        if intensity > max_intensity { max_intensity = intensity; }
        if intensity < min_intensity { min_intensity = intensity; }
        total_intensity +=  (count as usize) * i ;
        // guessing that a gaussian dist centered at 127.5
        // will have exceptional pixels within 1 stddev of min and max
        if intensity < 43 { qattrs.dark_pixel_count += count as u32; }
        else if intensity > (u8::MAX - 43) { qattrs.bright_pixel_count += count as u32; }

        let diff = count - prev_count;
        if diff > spike_threshold {
          // a spike is any sudden change in count for histogram bucket
          qattrs.hist_spike_count += 1;
        }
        prev_count = count;

        // histogram spreading calculation
        cumulative_count += count as usize;
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
  // TODO should we actually use (max_intensity - min_intensity) for divisor (range)?
  let hist_spread = (quartile_distance as f64) / 255.0;
  let mean_intensity = (total_intensity as f32 / total_pixels as f32) as u8;

  qattrs.mean_intensity = mean_intensity;
  qattrs.hist_spread = hist_spread;
  // qattrs.raw_histogram = channel_hist;
}



/// Measure the no-reference quality attributes of an image
pub fn analyze_image(img: &GrayImage)  -> MonoImageQAttributes {
  let mut durations: Vec<u32> = Vec::new();
  let mut tsms:i64 = 0;

  let mut qattrs = MonoImageQAttributes::default();
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
  hist_mean_spread_flatness(&img, &mut qattrs);
  // println!("{} << end histo ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start corners ",  timest(&mut tsms));
  qattrs.corner_count_f12 = count_corners_fast12(&img);
  // println!("{} << end corners ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start corners ",  timest(&mut tsms));
  qattrs.corner_count_f9 = count_corners_fast9(&img);
  // println!("{} << end corners ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  // println!("analyze durations {:?}", durations);

  qattrs
}

/// Measure the key no-reference quality attributes of an image -- fast
pub fn fast_analyze_image(img: &GrayImage)  -> MonoImageQAttributes {
  let mut durations: Vec<u32> = Vec::new();
  let mut tsms:i64 = 0;

  let mut qattrs = MonoImageQAttributes::default();
  qattrs.width = img.width();
  qattrs.height = img.height();

  // timest(&mut tsms);
  // // // println!("{} >> start sharpness ",  timest(&mut tsms));
  // let (_laplace_img, sharpness ) = laplacian_variance(&img);
  // qattrs.sharpness = sharpness;
  // // // println!("{} << end sharpness ",  timest(&mut tsms));
  // timex(&mut tsms, &mut durations);

  timest(&mut tsms);
  // println!("{} >> start histo ",  timest(&mut tsms));
  fast_histogram_analysis(&img, &mut qattrs);
  // println!("{} << end histo ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  // For fast analysis, limit to one or the other corner detector?
  timest(&mut tsms);
  // println!("{} >> start corners ",  timest(&mut tsms));
  qattrs.corner_count_f12 = count_corners_fast12(&img);
  // println!("{} << end corners ",  timest(&mut tsms));
  timex(&mut tsms, &mut durations);

  // timest(&mut tsms);
  // // println!("{} >> start corners ",  timest(&mut tsms));
  // qattrs.corner_count_f9 = count_corners_fast9(&img);
  // // println!("{} << end corners ",  timest(&mut tsms));
  // timex(&mut tsms, &mut durations);

  // println!("analyze durations {:?}", durations);

  qattrs
}

/// Estimate the maximum number of corners that could be detected in an
/// image with the given dimensions, with the estimated corner density.
///  - `density` is the ratio of non-corner pixels to corner pixels, say 25 for FAST-12
pub fn estimate_max_corners_fast12(width: u32, height: u32, density: u32) -> u32
{
  // the border for FAST-12 is essentially 3 pixels wide
  assert_ne!(density, 0); assert!(width > 6); assert!(height > 6);
  (width - 6) * (height - 6) / density
}

/// Count the number of FAST12 corners in an image
pub fn count_corners_fast12(img: &GrayImage) -> u32 {
  let all_corners = corners_fast12(&img, 32);
  all_corners.len() as u32
}

/// Count the number of FAST9 corners in an image
pub fn count_corners_fast9(img: &GrayImage) -> u32 {
  let all_corners = corners_fast9(&img, 32);
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