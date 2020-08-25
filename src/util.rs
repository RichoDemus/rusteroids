use nalgebra::Vector2;
use quicksilver::geom;

pub(crate) fn convert(vec: geom::Vector) -> Vector2<f32> {
    Vector2::new(vec.x, vec.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector2;

    #[test]
    fn vector_from_vector() {
        let vector: geom::Vector = geom::Vector { x: 10., y: 20. };
        let result: Vector2<f32> = convert(vector);
        assert_eq!(Vector2::new(20., 10.), result);
    }
}
