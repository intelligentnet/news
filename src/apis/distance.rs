pub fn cosine_dist(a: &[f32], b: &[f32]) -> f32 {
    assert!(a.len() == b.len());
    let dot: f32 = a.iter().zip(b.iter()).fold(0.0, |a, (x, y)| a + x * y);
    let ma: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    1.0 - dot / ma * mb
}

pub fn euclidian_dist(a: &[f32], b: &[f32]) -> f32 {
    assert!(a.len() == b.len());
    a.iter().zip(b).fold(0.0, |a, (x, y)| { let d = x - y; a + d * d }).sqrt()
//    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidian_dist_same_points() {
        let point = [1.0, 2.0, 3.0];
        assert_eq!(euclidian_dist(&point, &point), 0.0);
    }

    #[test]
    fn test_euclidian_dist_different_points() {
        let point_a = [1.0, 2.0, 3.0];
        let point_b = [4.0, 6.0, 8.0];
        let expected_distance = 50.0_f32.sqrt();
        assert!((euclidian_dist(&point_a, &point_b) - expected_distance).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic]
    fn test_euclidian_dist_different_dimensions() {
        let point_a = [1.0, 2.0, 3.0];
        let point_b = [4.0, 6.0];
        euclidian_dist(&point_a, &point_b);
    }

    #[test]
    fn test_cosine_dist_identical() {
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        assert!((cosine_dist(&a, &b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_dist_orthogonal() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        assert!((cosine_dist(&a, &b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_dist_opposite() {
        let a = [1.0, 0.0, 0.0];
        let b = [-1.0, 0.0, 0.0];
        assert!((cosine_dist(&a, &b) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic]
    fn test_cosine_dist_different_lengths() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0];
        assert!((cosine_dist(&a, &b) - 1.0).abs() < f32::EPSILON);
    }
}
