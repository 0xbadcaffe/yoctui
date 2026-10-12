//! Patience anchors with a bounded LCS for small ambiguous regions.
//! The fallback is an exact replace region, never a truncated comparison.
use super::*;

pub fn compare_file_text(
    paths: [PathBuf; 2],
    left: &str,
    right: &str,
) -> Result<FileDiffComparison, String> {
    let a: Vec<_> = left.split_inclusive('\n').collect();
    let b: Vec<_> = right.split_inclusive('\n').collect();
    if a.len() > FILE_DIFF_MAX_LINES || b.len() > FILE_DIFF_MAX_LINES {
        return Err(format!(
            "Comparison supports at most {FILE_DIFF_MAX_LINES} lines per file."
        ));
    }
    let mut pairs = Vec::new();
    let mut budget = 1_000_000usize;
    matches(&a, &b, 0, 0, &mut pairs, &mut budget, 0);
    let mut rows = Vec::new();
    let (mut ai, mut bi) = (0, 0);
    for (an, bn) in pairs.into_iter().chain(std::iter::once((a.len(), b.len()))) {
        let size = (an - ai).max(bn - bi);
        for offset in 0..size {
            rows.push(FileDiffRow {
                left: (ai + offset < an).then(|| cell(ai + offset, a[ai + offset])),
                right: (bi + offset < bn).then(|| cell(bi + offset, b[bi + offset])),
                equal: false,
            });
        }
        if an < a.len() {
            rows.push(FileDiffRow {
                left: Some(cell(an, a[an])),
                right: Some(cell(bn, b[bn])),
                equal: true,
            });
        }
        ai = an + 1;
        bi = bn + 1;
    }
    let changed_rows = rows.iter().filter(|row| !row.equal).count();
    Ok(FileDiffComparison {
        paths,
        rows,
        identical: left == right,
        final_newline: [left.ends_with('\n'), right.ends_with('\n')],
        changed_rows,
    })
}

fn cell(index: usize, text: &str) -> FileDiffCell {
    FileDiffCell {
        number: index + 1,
        text: text.strip_suffix('\n').unwrap_or(text).to_owned(),
    }
}

fn matches(
    a: &[&str],
    b: &[&str],
    mut ai: usize,
    mut bi: usize,
    out: &mut Vec<(usize, usize)>,
    budget: &mut usize,
    depth: usize,
) {
    let (mut a, mut b) = (a, b);
    while !a.is_empty() && !b.is_empty() && a[0] == b[0] {
        out.push((ai, bi));
        a = &a[1..];
        b = &b[1..];
        ai += 1;
        bi += 1;
    }
    let mut suffix = 0;
    while suffix < a.len().min(b.len()) && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix] {
        suffix += 1;
    }
    let (amid, bmid) = (&a[..a.len() - suffix], &b[..b.len() - suffix]);
    if !amid.is_empty() && !bmid.is_empty() && depth < 64 {
        let anchors = unique_anchors(amid, bmid);
        if !anchors.is_empty() {
            let (mut ap, mut bp) = (0, 0);
            for (an, bn) in anchors {
                matches(
                    &amid[ap..an],
                    &bmid[bp..bn],
                    ai + ap,
                    bi + bp,
                    out,
                    budget,
                    depth + 1,
                );
                out.push((ai + an, bi + bn));
                ap = an + 1;
                bp = bn + 1;
            }
            matches(
                &amid[ap..],
                &bmid[bp..],
                ai + ap,
                bi + bp,
                out,
                budget,
                depth + 1,
            );
        } else {
            let work = amid.len().saturating_mul(bmid.len());
            if work <= *budget {
                *budget -= work;
                lcs(amid, bmid, ai, bi, out);
            }
        }
    }
    for offset in 0..suffix {
        out.push((ai + amid.len() + offset, bi + bmid.len() + offset));
    }
}

fn unique_anchors(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    fn counts<'a>(lines: &[&'a str]) -> HashMap<&'a str, (usize, usize)> {
        let mut result = HashMap::new();
        for (index, line) in lines.iter().enumerate() {
            let entry = result.entry(*line).or_insert((0, index));
            entry.0 += 1;
        }
        result
    }
    let ac = counts(a);
    let bc = counts(b);
    let pairs: Vec<_> = a
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let &(count, other) = bc.get(line)?;
            (count == 1 && ac[line].0 == 1).then_some((index, other))
        })
        .collect();
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![None; pairs.len()];
    for (index, &(_, position)) in pairs.iter().enumerate() {
        let slot = tails.partition_point(|&tail| pairs[tail].1 < position);
        if slot > 0 {
            previous[index] = Some(tails[slot - 1]);
        }
        if slot == tails.len() {
            tails.push(index);
        } else {
            tails[slot] = index;
        }
    }
    let mut result = Vec::new();
    let mut cursor = tails.last().copied();
    while let Some(index) = cursor {
        result.push(pairs[index]);
        cursor = previous[index];
    }
    result.reverse();
    result
}

fn lcs(a: &[&str], b: &[&str], ai: usize, bi: usize, out: &mut Vec<(usize, usize)>) {
    let width = b.len() + 1;
    let mut matrix = vec![0u32; (a.len() + 1) * width];
    for x in (0..a.len()).rev() {
        for y in (0..b.len()).rev() {
            matrix[x * width + y] = if a[x] == b[y] {
                matrix[(x + 1) * width + y + 1] + 1
            } else {
                matrix[(x + 1) * width + y].max(matrix[x * width + y + 1])
            };
        }
    }
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        if a[x] == b[y] {
            out.push((ai + x, bi + y));
            x += 1;
            y += 1;
        } else if matrix[(x + 1) * width + y] >= matrix[x * width + y + 1] {
            x += 1;
        } else {
            y += 1;
        }
    }
}
