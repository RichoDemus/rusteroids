use nalgebra::Vector2;
use quicksilver::geom;

pub(crate) fn convert(vec: geom::Vector) -> Vector2<f64> {
    Vector2::new(vec.x.into(), vec.y.into())
}
