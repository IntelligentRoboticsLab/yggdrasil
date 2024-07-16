#[derive(Debug, Clone, Copy)]
pub struct BBox<T> {
    bbox: (f32, f32, f32, f32),
    _marker: std::marker::PhantomData<T>,
}

impl<T> BBox<T> {
    /// Create a new bounding box from the given coordinates.
    fn new(bbox: (f32, f32, f32, f32)) -> Self {
        BBox {
            bbox,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> BBox<T>
where
    BBox<T>: IntoBbox<Xyxy>,
{
    /// Compute the area of the bounding box.
    pub fn area(&self) -> f32 {
        let (x1, y1, x2, y2) = IntoBbox::<Xyxy>::into_bbox(self).bbox;
        (x2 - x1) * (y2 - y1)
    }

    /// Compute the intersection area between two bounding boxes.
    ///
    /// The intersection area is computed as the area of the overlap between the two bounding boxes.
    /// If the bounding boxes do not overlap, the intersection area is `0.0`.
    pub fn intersection<S: IntoBbox<Xyxy>>(&self, other: &S) -> f32
    where
        S: IntoBbox<Xyxy>,
    {
        let (x1, y1, x2, y2) = IntoBbox::<Xyxy>::into_bbox(self).bbox;
        let (x3, y3, x4, y4) = IntoBbox::<Xyxy>::into_bbox(other).bbox;

        let x1 = x1.max(x3);
        let y1 = y1.max(y3);
        let x2 = x2.min(x4);
        let y2 = y2.min(y4);

        if x2 < x1 || y2 < y1 {
            0.0
        } else {
            (x2 - x1) * (y2 - y1)
        }
    }

    /// Compute the union area between two bounding boxes.
    ///
    /// The union area is computed as the sum of the areas of the two bounding boxes minus the
    /// intersection area.
    pub fn union<S: IntoBbox<Xyxy>>(&self, other: &S) -> f32
    where
        S: IntoBbox<Xyxy>,
    {
        let area1 = IntoBbox::<Xyxy>::into_bbox(self).area();
        let area2 = IntoBbox::<Xyxy>::into_bbox(other).area();
        area1 + area2 - self.intersection(other)
    }
}

/// Trait for converting a bounding box to a different representation.
pub trait IntoBbox<T> {
    fn into_bbox(&self) -> BBox<T>;
}

/// Marker type for bounding boxes with coordinates of the top-left and bottom-right corners.
#[derive(Debug, Clone, Copy)]
pub struct Xyxy;

impl BBox<Xyxy> {
    /// Create a bounding box from the coordinates of the top-left and bottom-right corners.
    pub fn xyxy(bbox: (f32, f32, f32, f32)) -> BBox<Xyxy> {
        BBox::new(bbox)
    }
}

impl IntoBbox<Xyxy> for BBox<Xyxy> {
    fn into_bbox(&self) -> BBox<Xyxy> {
        *self
    }
}

impl IntoBbox<Xywh> for BBox<Xyxy> {
    fn into_bbox(&self) -> BBox<Xywh> {
        let (x1, y1, x2, y2) = self.bbox;
        BBox::new((x1, y1, x2 - x1, y2 - y1))
    }
}

/// Marker type for bounding boxes with coordinates of the top-left corner and the width and height.
pub struct Xywh;

impl BBox<Xywh> {
    /// Create a bounding box from the coordinates of the top-left corner and the width and height.
    pub fn xywh(bbox: (f32, f32, f32, f32)) -> BBox<Xywh> {
        BBox::new(bbox)
    }
}

impl IntoBbox<Xyxy> for BBox<Xywh> {
    fn into_bbox(&self) -> BBox<Xyxy> {
        let (x, y, w, h) = self.bbox;
        BBox::new((x, y, x + w, y + h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox() {
        let bbox1 = BBox::xyxy((0.0, 0.0, 10.0, 10.0));
        let bbox2 = BBox::xyxy((5.0, 5.0, 15.0, 15.0));

        assert_eq!(bbox1.intersection(&bbox2), 25.0);
        assert_eq!(bbox1.union(&bbox2), 175.0);
    }
}
