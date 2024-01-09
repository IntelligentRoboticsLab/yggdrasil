use rand::seq::SliceRandom;

use super::segmentation::Segment;

use super::Line;

impl Line {
    fn new(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        Line { x1, y1, x2, y2 }
    }
}

pub fn fit_line_ransac(
    points: &Vec<Segment>,
    min_samples: usize,
    residual_threshold: f64,
    max_trials: usize,
) -> ((f64, f64), Vec<bool>) {
    let mut best_model: Option<(f64, f64)> = None;
    let mut best_inliers: Option<Vec<bool>> = None;

    // for point in points {
    //     println!("{:?}, {:?}", point.x, point.y);
    // }

    for _ in 0..max_trials {
        // Randomly sample points to form a model
        let sample: Vec<Segment> = points
            .choose_multiple(&mut rand::thread_rng(), min_samples)
            .cloned()
            .collect();

        // Fit a model (line in this case) using the sampled points
        let model = polyfit(&sample);

        // Calculate residuals and find inliers
        let residuals: Vec<f64> = points
            .iter()
            .map(|p| (polyval(&model, p.x) as f64 - p.y as f64).abs())
            .collect();
        let inliers: Vec<bool> = residuals.iter().map(|&r| r < residual_threshold).collect();

        // Update the best model if the current model has more inliers
        if best_inliers.is_none()
            || inliers.iter().filter(|&&b| b).count()
                > best_inliers.clone().unwrap().iter().filter(|&&b| b).count()
        {
            best_model = Some(model);
            best_inliers = Some(inliers);
        }
    }

    (best_model.unwrap(), best_inliers.unwrap())
}

// Fit a line using least squares
fn polyfit(points: &Vec<Segment>) -> (f64, f64) {
    let n = points.len();
    let sum_x: f64 = points.iter().map(|p| p.x as f64).sum();
    let sum_y: f64 = points.iter().map(|p| p.y as f64).sum();
    let sum_x_squared: f64 = points.iter().map(|p| (p.x as f64) * (p.x as f64)).sum();
    let sum_xy: f64 = points.iter().map(|p| (p.x as f64) * (p.y as f64)).sum();

    let a = (n as f64 * sum_xy - sum_x * sum_y) / (n as f64 * sum_x_squared - sum_x * sum_x);
    let b = (sum_y - a * sum_x) / n as f64;

    (a, b)
}

fn polyval(coefficients: &(f64, f64), x: u32) -> u32 {
    (coefficients.0 * x as f64 + coefficients.1) as u32
}

pub fn fit_lines(segments: &Vec<Segment>) -> Vec<Line> {
    let mut leftover_points: Vec<Segment> = segments.clone();
    leftover_points.sort_by(|a, b| a.x.cmp(&b.x));
    let mut lines: Vec<Line> = Vec::new();

    while leftover_points.len() > 5 {
        // Robustly fit line only using inlier data with RANSAC algorithm
        let (model, inliers) = fit_line_ransac(&leftover_points, 4, 20.0, 1000);

        // Indexes within the leftover_points vector that are outliers
        let outliers: Vec<bool> = inliers.iter().map(|&b| !b).collect();

        // Filter out inlier data
        let inlier_data = leftover_points
            .iter()
            .zip(inliers)
            .filter(|(_, b)| *b)
            .map(|(p, _)| *p)
            .collect::<Vec<Segment>>();

        // Calculate line points using the model
        let line_x: Vec<u32> = ((inlier_data[0].x)..=(inlier_data.last().unwrap().x)).collect();
        let line_y_robust: Vec<u32> = line_x.iter().map(|&x| polyval(&model, x)).collect();

        // Plotting code is omitted
        if line_x.len() > 0 {
            lines.push(Line::new(
                line_x[0],
                line_y_robust[0],
                *line_x.last().unwrap(),
                *line_y_robust.last().unwrap(),
            ));
            // Print model
            // println!("Model: {:?}", model);
        } else {
            println!("Line_x is empty");
            println!("Inlier data: {:?}", inlier_data);
        }

        leftover_points = leftover_points
            .into_iter()
            .zip(outliers)
            .filter(|(_, b)| *b)
            .map(|(p, _)| p)
            .collect();
    }

    lines
}
