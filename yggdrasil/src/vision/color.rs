/// Converts YUV to RGB using the BT.601 conversion matrix.
///
/// BT.601 (aka. SDTV, aka. Rec.601). wiki: <https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.601_conversion>
pub fn yuv_to_rgb_bt601((y, u, v): (u8, u8, u8)) -> (u8, u8, u8) {
    yuv_to_rgb((y, u, v), partials_bt601)
}

/// Converts YUV to RGB using the BT.709 conversion matrix.
///
/// BT.709 (aka. HDTV, aka. Rec.709). wiki: <https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.709_conversion>.
pub fn yuv_to_rgb_bt709((y, u, v): (u8, u8, u8)) -> (u8, u8, u8) {
    yuv_to_rgb((y, u, v), partials_bt709)
}

/// Converts YUY2 to RGB using the BT.601 conversion matrix.
///     
/// BT.601 (aka. SDTV, aka. Rec.601). wiki: <https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.601_conversion>
pub fn yuy2_to_rgb_bt601((y1, u, y2, v): (u8, u8, u8, u8)) -> ((u8, u8, u8), (u8, u8, u8)) {
    yuy2_to_rgb((y1, u, y2, v), partials_bt601)
}

/// Converts YUY2 to RGB using the BT.709 conversion matrix.
///     
/// BT.709 (aka. HDTV, aka. Rec.709). wiki: <https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.709_conversion>
pub fn yuy2_to_rgb_bt709((y1, u, y2, v): (u8, u8, u8, u8)) -> ((u8, u8, u8), (u8, u8, u8)) {
    yuy2_to_rgb((y1, u, y2, v), partials_bt709)
}

#[inline]
fn yuv_to_rgb(
    (y, u, v): (u8, u8, u8),
    partial: impl Fn(f32, f32) -> (f32, f32, f32),
) -> (u8, u8, u8) {
    let (y, u, v) = (y as f32, u as f32, v as f32);

    // rescale YUV values
    let y = (y - 16.0) / 219.0;
    let u = (u - 128.0) / 224.0;
    let v = (v - 128.0) / 224.0;

    let (r_partial, g_partial, b_partial) = partial(u, v);

    // let (r_partial, g_partial, b_partial) = partials_bt709(u, v);

    let r = y + r_partial;
    let g = y + g_partial;
    let b = y + b_partial;

    (
        (255.0 * r).clamp(0.0, 255.0) as u8,
        (255.0 * g).clamp(0.0, 255.0) as u8,
        (255.0 * b).clamp(0.0, 255.0) as u8,
    )
}

#[inline]
fn yuy2_to_rgb(
    (y1, u, y2, v): (u8, u8, u8, u8),
    partial: impl Fn(f32, f32) -> (f32, f32, f32),
) -> ((u8, u8, u8), (u8, u8, u8)) {
    let (y1, u, y2, v) = (y1 as f32, u as f32, y2 as f32, v as f32);

    // rescale YUV values
    let y1 = (y1 - 16.0) / 219.0;
    let u = (u - 128.0) / 224.0;
    let y2 = (y2 - 16.0) / 219.0;
    let v = (v - 128.0) / 224.0;

    let (r_partial, g_partial, b_partial) = partial(u, v);

    let r1 = y1 + r_partial;
    let g1 = y1 - g_partial;
    let b1 = y1 + b_partial;

    let r2 = y2 + r_partial;
    let g2 = y2 - g_partial;
    let b2 = y2 + b_partial;

    (
        (
            (255.0 * r1).clamp(0.0, 255.0) as u8,
            (255.0 * g1).clamp(0.0, 255.0) as u8,
            (255.0 * b1).clamp(0.0, 255.0) as u8,
        ),
        (
            (255.0 * r2).clamp(0.0, 255.0) as u8,
            (255.0 * g2).clamp(0.0, 255.0) as u8,
            (255.0 * b2).clamp(0.0, 255.0) as u8,
        ),
    )
}

#[inline]
fn partials_bt601(u: f32, v: f32) -> (f32, f32, f32) {
    (1.402 * v, -0.344 * u - 0.714 * v, 1.772 * u)
}

#[inline]
fn partials_bt709(u: f32, v: f32) -> (f32, f32, f32) {
    (1.575 * v, -0.187 * u - 0.468 * v, 1.856 * u)
}
