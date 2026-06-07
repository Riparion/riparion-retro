//! Number display helpers shared by the games.

/// Format money the way the 8-bit classics did: plain grouped integer under a
/// million, otherwise three significant figures with a magnitude word.
pub fn fmt_money(v: f64) -> String {
    let neg = v < 0.0;
    let v = v.abs().floor();
    let s = if v < 1_000_000.0 {
        group_thousands(v as i64)
    } else {
        let units = [(1e12, "Trillion"), (1e9, "Billion"), (1e6, "Million")];
        let (scale, word) = units
            .iter()
            .find(|(scale, _)| v >= *scale)
            .copied()
            .unwrap_or((1e6, "Million"));
        let n = v / scale;
        if n >= 100.0 {
            format!("{n:.0} {word}")
        } else if n >= 10.0 {
            format!("{n:.1} {word}")
        } else {
            format!("{n:.2} {word}")
        }
    };
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// 1234567 → "1,234,567".
pub fn group_thousands(mut n: i64) -> String {
    let neg = n < 0;
    n = n.abs();
    let mut parts = Vec::new();
    loop {
        if n < 1000 {
            parts.push(n.to_string());
            break;
        }
        parts.push(format!("{:03}", n % 1000));
        n /= 1000;
    }
    parts.reverse();
    let s = parts.join(",");
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_formatting() {
        assert_eq!(fmt_money(0.0), "0");
        assert_eq!(fmt_money(999_999.0), "999,999");
        assert_eq!(fmt_money(1_230_000.0), "1.23 Million");
        assert_eq!(fmt_money(12_300_000.0), "12.3 Million");
        assert_eq!(fmt_money(1.23e9), "1.23 Billion");
        assert_eq!(fmt_money(-5000.0), "-5,000");
    }
}
