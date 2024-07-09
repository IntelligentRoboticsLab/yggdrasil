use ndarray::{Array, Array1, ArrayD, IxDyn};
use std::error::Error;

use itertools::Itertools;

#[derive(PartialEq)]
pub enum Indexing {
    Xy,
    Ij,
}

pub fn meshgrid<T>(xi: &[Array1<T>], indexing: Indexing) -> Result<Vec<ArrayD<T>>, Box<dyn Error>>
where
    T: Copy,
{
    let ndim = xi.len();
    let product = xi.iter().map(|x| x.iter()).multi_cartesian_product();

    let mut grids: Vec<ArrayD<T>> = Vec::with_capacity(ndim);

    for (dim_index, _) in xi.iter().enumerate() {
        // Generate a flat vector with the correct repeated pattern
        let values: Vec<T> = product.clone().map(|p| *p[dim_index]).collect();

        let mut grid_shape: Vec<usize> = vec![1; ndim];
        grid_shape[dim_index] = xi[dim_index].len();

        // Determine the correct repetition for each dimension
        for (j, len) in xi.iter().map(|x| x.len()).enumerate() {
            if j != dim_index {
                grid_shape[j] = len;
            }
        }

        let grid = Array::from_shape_vec(IxDyn(&grid_shape), values)?;
        grids.push(grid);
    }

    // Swap axes for "xy" indexing
    if matches!(indexing, Indexing::Xy) && ndim > 1 {
        for grid in &mut grids {
            grid.swap_axes(0, 1);
        }
    }

    Ok(grids)
}
