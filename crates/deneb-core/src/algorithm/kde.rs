//! Kernel Density Estimation (KDE)
//!
//! Computes a smooth density curve from a set of data points using a Gaussian
//! kernel with Silverman's rule-of-thumb bandwidth.

/// Calculate the mean (average) of a dataset.
///
/// Returns `None` if the slice is empty.
fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    Some(data.iter().sum::<f64>() / data.len() as f64)
}

/// Calculate the standard deviation (sample, Bessel-corrected) of a dataset.
///
/// Returns `None` if the slice has fewer than 2 elements.
fn std_dev(data: &[f64]) -> Option<f64> {
    if data.len() < 2 {
        return None;
    }
    let mean_val = mean(data)?;
    let variance: f64 =
        data.iter().map(|&x| (x - mean_val).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
    Some(variance.sqrt())
}

/// Calculate the extent (min, max) of a dataset.
///
/// Returns `None` if the slice is empty.
fn extent(data: &[f64]) -> Option<(f64, f64)> {
    if data.is_empty() {
        return None;
    }
    let mut min = data[0];
    let mut max = data[0];
    for &v in &data[1..] {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    Some((min, max))
}

/// Gaussian kernel function.
///
/// Evaluates the standard Gaussian (normal) density at `z = (x - xi) / h`,
/// already scaled by the normalisation factor `1 / (h * sqrt(2π))`.
fn gaussian_kernel(x: f64, xi: f64, h: f64) -> f64 {
    let z = (x - xi) / h;
    (-0.5 * z * z).exp() / (h * (2.0 * std::f64::consts::PI).sqrt())
}

/// Estimate density using a Gaussian kernel with Silverman's rule-of-thumb
/// bandwidth.
///
/// # Arguments
///
/// * `data` — input values (must contain ≥ 2 unique-ish values)
/// * `n_points` — number of equally-spaced evaluation points on the x-axis
///
/// # Returns
///
/// A vector of `(x, density)` pairs, or `None` if `data` has fewer than 2
/// elements or zero standard deviation.
///
/// # Algorithm
///
/// Bandwidth is chosen via Silverman's rule: `h = 1.06 · σ · n^(-1/5)`.
/// The evaluation grid extends 3 bandwidths beyond the data range on each
/// side to capture the tails.
pub fn gaussian_kde(data: &[f64], n_points: usize) -> Option<Vec<(f64, f64)>> {
    if data.len() < 2 || n_points == 0 {
        return None;
    }
    let sd = std_dev(data)?;
    if sd <= 0.0 {
        return None;
    }

    let n = data.len() as f64;
    let h = 1.06 * sd * n.powf(-0.2); // Silverman's rule of thumb
    let (min_val, max_val) = extent(data)?;

    let x_lo = min_val - 3.0 * h;
    let x_hi = max_val + 3.0 * h;
    let norm_factor = 1.0 / n; // per-point averaging

    let step = (x_hi - x_lo) / (n_points - 1).max(1) as f64;

    let result: Vec<(f64, f64)> = (0..n_points)
        .map(|i| {
            let x = x_lo + step * i as f64;
            let density: f64 =
                data.iter().map(|&xi| gaussian_kernel(x, xi, h)).sum::<f64>() * norm_factor;
            (x, density)
        })
        .collect();

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_mean_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let avg = mean(&data).unwrap();
        assert!((avg - 3.0).abs() < EPSILON);
    }

    #[test]
    fn test_mean_empty() {
        assert!(mean(&[]).is_none());
    }

    #[test]
    fn test_std_dev_basic() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&data).unwrap();
        assert!((sd - 2.138).abs() < 0.01);
    }

    #[test]
    fn test_std_dev_insufficient_data() {
        assert!(std_dev(&[5.0]).is_none());
    }

    #[test]
    fn test_extent_basic() {
        let data = vec![1.0, 5.0, 3.0, 9.0, 2.0];
        let (min, max) = extent(&data).unwrap();
        assert!((min - 1.0).abs() < EPSILON);
        assert!((max - 9.0).abs() < EPSILON);
    }

    #[test]
    fn test_extent_empty() {
        assert!(extent(&[]).is_none());
    }

    #[test]
    fn test_kde_basic() {
        let data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let result = gaussian_kde(&data, 50).unwrap();
        assert_eq!(result.len(), 50);
        // All densities must be non-negative
        assert!(result.iter().all(|&(_, y)| y >= 0.0));
    }

    #[test]
    fn test_kde_too_few_points() {
        assert!(gaussian_kde(&[1.0], 50).is_none());
    }

    #[test]
    fn test_kde_zero_n_points() {
        assert!(gaussian_kde(&[1.0, 2.0], 0).is_none());
    }

    #[test]
    fn test_kde_empty() {
        assert!(gaussian_kde(&[], 50).is_none());
    }

    #[test]
    fn test_kde_peak_near_data_center() {
        // Symmetric data: peak density should be near the center (x ≈ 0)
        let data: Vec<f64> = (0..11).map(|i| i as f64 - 5.0).collect();
        let result = gaussian_kde(&data, 101).unwrap();
        let (peak_x, _) = result
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        assert!(peak_x.abs() < 1.0, "Peak should be near x=0, got {peak_x}");
    }

    #[test]
    fn test_kde_constant_data_returns_none() {
        // Zero standard deviation → None
        assert!(gaussian_kde(&[5.0, 5.0, 5.0, 5.0], 50).is_none());
    }

    #[test]
    fn test_kde_x_values_monotonic() {
        let data: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let result = gaussian_kde(&data, 30).unwrap();
        for w in result.windows(2) {
            assert!(w[0].0 < w[1].0, "x values must be strictly increasing");
        }
    }

    #[test]
    fn test_kde_integral_approximately_one() {
        // The area under the density curve should be ≈ 1.0
        let data: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
        let result = gaussian_kde(&data, 200).unwrap();
        let area: f64 = result.windows(2).map(|w| {
            let dx = w[1].0 - w[0].0;
            let avg_y = (w[0].1 + w[1].1) / 2.0;
            dx * avg_y
        }).sum();
        assert!(
            (area - 1.0).abs() < 0.05,
            "Area under KDE should be ≈ 1.0, got {area}"
        );
    }
}
