// Copyright(c) The Maintainers of Nanvix.
// Licensed by the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
use ::alloc::{
    vec,
    vec::Vec,
};
use ::fastrand::Rng;
use ::micromath::F32Ext;
use ::nvx::sys::error::Error;
use ::serde::Deserialize;
use ::serde_json::de::from_str;

//==================================================================================================
// Structs
//==================================================================================================

#[derive(Deserialize)]
struct Parameters {
    matrix_size: usize,
    seed: u64,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            matrix_size: 32,
            seed: 32,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
fn main() -> Result<(), Error> {
    let raw_content: Option<&str> = option_env!("CONFIG");

    let params: Parameters = if let Some(raw_content) = raw_content {
        from_str(raw_content).expect("failed to parse CONFIG environment variable")
    } else {
        Parameters::default()
    };

    let mut rng: Rng = Rng::with_seed(params.seed);

    // Calculate the safe range for random values
    let max: f32 = (i32::MAX as u64 / params.matrix_size as u64) as f32;
    let max_u32_value: u32 = max.sqrt().abs() as u32;
    let safe_range: core::ops::RangeInclusive<u32> = 0..=max_u32_value;

    // Representing a matrix linearly.
    let vec_size: usize = params.matrix_size * params.matrix_size;

    let mut m1: Vec<i32> = Vec::with_capacity(vec_size);
    for _ in 0..vec_size {
        let curr: u32 = rng.u32(safe_range.clone());
        m1.push(curr as i32);
    }

    let mut m2: Vec<i32> = Vec::with_capacity(vec_size);
    for _ in 0..vec_size {
        let curr: u32 = rng.u32(safe_range.clone());
        m2.push(curr as i32);
    }

    let mut result_matrix: Vec<i32> = vec![0; vec_size];

    // Execute cache-oblivious matrix multiplication.
    matrix_mult(
        &m1,
        &m2,
        &mut result_matrix,
        params.matrix_size,
        params.matrix_size,
        0,
        0,
        0,
        0,
        0,
        0,
    );

    Ok(())
}

///
/// # Description
///
/// Executes matrix multiplication with a cache oblivious algorithm.
///
/// It divides the input matrices `m1` and `m2` into quadrants and recursively multiplies and adds
/// them.
///
#[allow(clippy::too_many_arguments)]
fn matrix_mult(
    m1: &[i32],
    m2: &[i32],
    res: &mut [i32],
    n: usize,
    matrix_size: usize,
    row_m1: usize,
    col_m1: usize,
    row_m2: usize,
    col_m2: usize,
    row_res: usize,
    col_res: usize,
) {
    if n <= 16 {
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let index_m1: usize = (row_m1 + i) * matrix_size + (col_m1 + k);
                    let index_m2: usize = (row_m2 + k) * matrix_size + (col_m2 + j);
                    let index_res: usize = (row_res + i) * matrix_size + (col_res + j);

                    debug_assert!(index_m1 < matrix_size * matrix_size, "index_m1 out of bounds");
                    debug_assert!(index_m2 < matrix_size * matrix_size, "index_m2 out of bounds");
                    debug_assert!(index_res < matrix_size * matrix_size, "index_res out of bounds");

                    let product: i64 = m1[index_m1] as i64 * m2[index_m2] as i64;
                    res[index_res] = (res[index_res] as i64 + product) as i32;
                }
            }
        }
        return;
    }

    if n <= 1 {
        return;
    }

    let half: usize = n / 2;

    // A11 * B11 to C11
    matrix_mult(m1, m2, res, half, matrix_size, row_m1, col_m1, row_m2, col_m2, row_res, col_res);

    // A12 * B21 to C11
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1,
        col_m1 + half,
        row_m2,
        col_m2,
        row_res,
        col_res,
    );

    // A11 * B12 to C12
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1,
        col_m1,
        row_m2,
        col_m2 + half,
        row_res,
        col_res + half,
    );

    // A12 * B22 to C12
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1,
        col_m1 + half,
        row_m2 + half,
        col_m2 + half,
        row_res,
        col_res + half,
    );

    // A21 * B11 to C21
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1 + half,
        col_m1,
        row_m2,
        col_m2,
        row_res + half,
        col_res,
    );

    // A22 * B12 to C21
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1 + half,
        col_m1 + half,
        row_m2 + half,
        col_m2,
        row_res + half,
        col_res,
    );

    // A21 * B12 to C22
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1 + half,
        col_m1,
        row_m2,
        col_m2 + half,
        row_res + half,
        col_res + half,
    );

    // A22 * B22 to C22
    matrix_mult(
        m1,
        m2,
        res,
        half,
        matrix_size,
        row_m1 + half,
        col_m1 + half,
        row_m2 + half,
        col_m2 + half,
        row_res + half,
        col_res + half,
    );
}
