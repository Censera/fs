use fsize::format_size;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let dec_units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
        let bin_units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

        for scale in 0..=4 {
            let dec_bytes = (1.556 * 1000f64.powi(scale as i32)) as u64;
            let exp_dec = if scale == 0 {
                format!("{} B", dec_bytes)
            } else {
                format!("1.55 {}", dec_units[scale])
            };

            assert_eq!(
                format_size(dec_bytes, None, false),
                exp_dec,
                "Failed decimal scale {}",
                dec_units[scale]
            );

            let bin_bytes = (1.556 * 1024f64.powi(scale as i32)) as u64;
            let exp_bin = if scale == 0 {
                format!("{} B", bin_bytes)
            } else {
                format!("1.55 {}", bin_units[scale])
            };

            assert_eq!(
                format_size(bin_bytes, None, true),
                exp_bin,
                "Failed binary scale {}",
                bin_units[scale]
            );
        }
    }
}
