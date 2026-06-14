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

/// Trim a float for display: integers bare, otherwise one decimal.
pub fn fmt_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{:.0}", x.round())
    } else {
        format!("{x:.1}")
    }
}

/// Money with grouped cents and a leading sign: "$1,234.50" / "-$1,234.50".
/// For ledgers and prompts that mirror the BASIC classics' `PRINT "$";I`.
pub fn fmt_dollars(v: f64) -> String {
    let cents = (v.abs() * 100.0).round() as i64;
    let body = format!("${}.{:02}", group_thousands(cents / 100), cents % 100);
    if v < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

/// Compact money with a leading sign for tight spots like status chips:
/// "$1,234" / "-$1,234" / "$1.23 Million" (uses [`fmt_money`]'s magnitude
/// words for large values).
pub fn fmt_dollars_compact(v: f64) -> String {
    let body = fmt_money(v.abs());
    if v < 0.0 {
        format!("-${body}")
    } else {
        format!("${body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_trimming() {
        assert_eq!(fmt_num(947.5), "947.5");
        assert_eq!(fmt_num(890.0), "890");
        assert_eq!(fmt_num(-35.0), "-35");
        assert_eq!(fmt_num(0.0), "0");
    }

    #[test]
    fn money_formatting() {
        assert_eq!(fmt_money(0.0), "0");
        assert_eq!(fmt_money(999_999.0), "999,999");
        assert_eq!(fmt_money(1_230_000.0), "1.23 Million");
        assert_eq!(fmt_money(12_300_000.0), "12.3 Million");
        assert_eq!(fmt_money(1.23e9), "1.23 Billion");
        assert_eq!(fmt_money(-5000.0), "-5,000");
    }

    #[test]
    fn dollar_formatting() {
        assert_eq!(fmt_dollars(600.0), "$600.00");
        assert_eq!(fmt_dollars(1234.5), "$1,234.50");
        assert_eq!(fmt_dollars(0.0), "$0.00");
        assert_eq!(fmt_dollars(-40.25), "-$40.25");
        assert_eq!(fmt_dollars_compact(645.0), "$645");
        assert_eq!(fmt_dollars_compact(-5000.0), "-$5,000");
        assert_eq!(fmt_dollars_compact(1_230_000.0), "$1.23 Million");
    }
}
