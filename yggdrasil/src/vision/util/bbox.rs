/// A type-safe bounding box.
///
/// It is a wrapper around a tuple of four `f32` values representing the coordinates of the bounding box.
/// The type parameter `T` is used to specify the format of the bounding box, and is used to enforce type safety.
///
/// # Conversion
///
/// The bounding box can be converted between different formats using the [`ConvertBbox`] trait.
/// This allows for easy conversion between different formats without having to manually convert the coordinates.
///
/// ```
/// use yggdrasil::vision::util::bbox::*;
///
/// let xyxy = Bbox::xyxy(4.0, 4.0, 10.0, 10.0);
/// let xywh: Bbox<Xywh> = xyxy.convert();
///
/// assert_eq!(xywh.inner, (4.0, 4.0, 6.0, 6.0));
/// ```
///
/// # Formats
///
/// The following formats are supported:
///
/// - [`Xyxy`] (xmin, ymin, xmax, ymax)
/// - [`Xywh`] (xmin, ymin, width, height)
/// - [`Cxcywh`] (center_x, center_y, width, height)
#[derive(Debug, Clone, Copy)]
pub struct Bbox<T> {
    pub inner: (f32, f32, f32, f32),
    _marker: std::marker::PhantomData<T>,
}

impl<T> Bbox<T> {
    /// Create a new bounding box from the given coordinates.
    fn new(bbox: (f32, f32, f32, f32)) -> Self {
        Bbox {
            inner: bbox,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Bbox<T>
where
    Bbox<T>: ConvertBbox<Xyxy>,
{
    /// Compute the area of the bounding box.
    pub fn area(&self) -> f32 {
        let (x1, y1, x2, y2) = ConvertBbox::<Xyxy>::convert(self).inner;
        (x2 - x1) * (y2 - y1)
    }

    /// Compute the intersection area between two bounding boxes.
    ///
    /// The intersection area is computed as the area of the overlap between the two bounding boxes.
    /// If the bounding boxes do not overlap, the intersection area is `0.0`.
    pub fn intersection<S>(&self, other: &S) -> f32
    where
        S: ConvertBbox<Xyxy>,
    {
        let (x1, y1, x2, y2) = ConvertBbox::<Xyxy>::convert(self).inner;
        let (x3, y3, x4, y4) = ConvertBbox::<Xyxy>::convert(other).inner;

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
    pub fn union<S>(&self, other: &S) -> f32
    where
        S: ConvertBbox<Xyxy>,
    {
        let area1 = ConvertBbox::<Xyxy>::convert(self).area();
        let area2 = ConvertBbox::<Xyxy>::convert(other).area();
        area1 + area2 - self.intersection(other)
    }

    /// Compute the intersection over union (IoU) between two bounding boxes.
    pub fn iou<S>(&self, other: &S) -> f32
    where
        S: ConvertBbox<Xyxy>,
    {
        let intersect = self.intersection(other);
        let union = self.union(other);
        intersect / union
    }
}

impl<T> From<Bbox<T>> for (f32, f32, f32, f32) {
    fn from(bbox: Bbox<T>) -> Self {
        bbox.inner
    }
}

/// Trait for converting a bounding box to a different representation.
pub trait ConvertBbox<T> {
    fn convert(&self) -> Bbox<T>;
}

/// Marker type for bounding boxes with coordinates of the top-left and bottom-right corners.
#[derive(Debug, Clone, Copy)]
pub struct Xyxy;

impl Bbox<Xyxy> {
    /// Create a bounding box from the coordinates of the top-left and bottom-right corners.
    pub fn xyxy(x1: f32, y1: f32, x2: f32, y2: f32) -> Bbox<Xyxy> {
        Bbox::new((x1, y1, x2, y2))
    }

    /// Clamp the bounding box to the given width and height.
    pub fn clamp(&self, width: f32, height: f32) -> Bbox<Xyxy> {
        let (x1, y1, x2, y2) = self.inner;
        let x1 = x1.max(0.0).min(width);
        let y1 = y1.max(0.0).min(height);
        let x2 = x2.max(0.0).min(width);
        let y2 = y2.max(0.0).min(height);
        Bbox::new((x1, y1, x2, y2))
    }

    /// Scale the bounding box to the given width and height.
    pub fn scaled(&self, width: f32, height: f32) -> Bbox<Xyxy> {
        let (x1, y1, x2, y2) = self.inner;
        Bbox::new((x1 * width, y1 * height, x2 * width, y2 * height))
    }
}

impl ConvertBbox<Xyxy> for Bbox<Xyxy> {
    fn convert(&self) -> Bbox<Xyxy> {
        *self
    }
}

impl ConvertBbox<Xywh> for Bbox<Xyxy> {
    fn convert(&self) -> Bbox<Xywh> {
        let (x1, y1, x2, y2) = self.inner;
        Bbox::new((x1, y1, x2 - x1, y2 - y1))
    }
}

impl ConvertBbox<Cxcywh> for Bbox<Xyxy> {
    fn convert(&self) -> Bbox<Cxcywh> {
        let (x1, y1, x2, y2) = self.inner;
        Bbox::new(((x1 + x2) / 2.0, (y1 + y2) / 2.0, x2 - x1, y2 - y1))
    }
}

/// Marker type for bounding boxes with coordinates of the top-left corner and the width and height
#[derive(Debug, Clone, Copy)]
pub struct Xywh;

impl Bbox<Xywh> {
    /// Create a bounding box from the coordinates of the top-left corner and the width and height.
    pub fn xywh(x: f32, y: f32, w: f32, h: f32) -> Bbox<Xywh> {
        Bbox::new((x, y, w, h))
    }

    /// Clamp the bounding box to the given width and height.
    pub fn clamp(&self, width: f32, height: f32) -> Bbox<Xywh> {
        let (x, y, w, h) = self.inner;
        let x = x.max(0.0).min(width);
        let y = y.max(0.0).min(height);
        let w = w.max(0.0).min(width - x);
        let h = h.max(0.0).min(height - y);
        Bbox::new((x, y, w, h))
    }
}

impl ConvertBbox<Xyxy> for Bbox<Xywh> {
    fn convert(&self) -> Bbox<Xyxy> {
        let (x, y, w, h) = self.inner;
        Bbox::new((x, y, x + w, y + h))
    }
}

impl ConvertBbox<Xywh> for Bbox<Xywh> {
    fn convert(&self) -> Bbox<Xywh> {
        *self
    }
}

/// Marker type for bounding boxes with coordinates of the center and the width and height.
#[derive(Debug, Clone, Copy)]
pub struct Cxcywh;

impl Bbox<Cxcywh> {
    /// Create a bounding box from the coordinates of the center and the width and height.
    pub fn cxcywh(cx: f32, cy: f32, w: f32, h: f32) -> Bbox<Cxcywh> {
        Bbox::new((cx, cy, w, h))
    }

    /// Clamp the bounding box to the given width and height.
    pub fn clamp(&self, width: f32, height: f32) -> Bbox<Cxcywh> {
        let (cx, cy, w, h) = self.inner;
        let x1 = (cx - w / 2.0).max(0.0).min(width);
        let y1 = (cy - h / 2.0).max(0.0).min(height);
        let x2 = (cx + w / 2.0).max(0.0).min(width);
        let y2 = (cy + h / 2.0).max(0.0).min(height);
        Bbox::new((x1, y1, x2, y2))
    }
}

impl ConvertBbox<Xyxy> for Bbox<Cxcywh> {
    fn convert(&self) -> Bbox<Xyxy> {
        let (cx, cy, w, h) = self.inner;
        Bbox::new((cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0))
    }
}

impl ConvertBbox<Xywh> for Bbox<Cxcywh> {
    fn convert(&self) -> Bbox<Xywh> {
        let (cx, cy, w, h) = self.inner;
        Bbox::new((cx - w / 2.0, cy - h / 2.0, w, h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iou_xyxy() {
        let bbox1 = Bbox::xyxy(0.0, 0.0, 10.0, 10.0);
        let bbox2 = Bbox::xyxy(5.0, 5.0, 15.0, 15.0);

        assert_eq!(bbox1.intersection(&bbox2), 25.0);
        assert_eq!(bbox1.union(&bbox2), 175.0);
        assert_eq!(bbox1.iou(&bbox2), 25.0 / 175.0);
    }

    #[test]
    fn iou_xywh() {
        let bbox1 = Bbox::xywh(0.0, 0.0, 10.0, 10.0);
        let bbox2 = Bbox::xywh(5.0, 5.0, 10.0, 10.0);

        assert_eq!(bbox1.intersection(&bbox2), 25.0);
        assert_eq!(bbox1.union(&bbox2), 175.0);
        assert_eq!(bbox1.iou(&bbox2), 25.0 / 175.0);
    }

    #[test]
    fn iou_cxcywh() {
        let bbox1 = Bbox::cxcywh(5.0, 5.0, 10.0, 10.0);
        let bbox2 = Bbox::cxcywh(10.0, 10.0, 10.0, 10.0);

        assert_eq!(bbox1.intersection(&bbox2), 25.0);
        assert_eq!(bbox1.union(&bbox2), 175.0);
        assert_eq!(bbox1.iou(&bbox2), 25.0 / 175.0);
    }
}
