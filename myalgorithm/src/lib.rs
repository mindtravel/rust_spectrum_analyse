use num_traits::{NumCast};

// pub fn add(a: i32, b: i32) -> i32 {
//     a + b
// }


pub const SAMPLE_RATE: f32 = 44100.0; 
pub const MAX_FREQ: f32 = SAMPLE_RATE / 2.0; 
pub const BUFFER_SZ: usize = 4096;
pub const BUFFER_SZ_HALF: usize = BUFFER_SZ / 2;

pub fn get_freq(idx: usize) -> f32{
    idx as f32 * SAMPLE_RATE / ((BUFFER_SZ/2) as f32) + 1.0
}

pub fn get_normalized_db<T>(db: T) -> f32
where
    T: NumCast,          // 支持从任意数值类型转换
{
    let db_float = NumCast::from(db).unwrap_or(0.0);
    (db_float + 90.0) / 90.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
