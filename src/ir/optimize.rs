//! # IR Optimizer
//!
//! Optimization passes that transform IR programs to reduce redundancy
//! and improve efficiency.
//!
//! ## Optimization Passes
//!
//! 1. **Remove redundant styles**: Don't emit SetBold(true) if already bold
//! 2. **Merge adjacent text**: Combine consecutive Text ops
//! 3. **Remove redundant init**: Only keep the first Init op
//! 4. **Eliminate dead style changes**: Remove style changes followed by reset

use super::ops::{Op, Program, StyleState};

impl Program {
    /// Apply all optimization passes.
    pub fn optimize(self) -> Self {
        let ops = self.ops;
        let ops = remove_redundant_init(ops);
        let ops = collapse_style_toggles(ops);
        let ops = remove_redundant_styles(ops);
        let ops = merge_adjacent_text(ops);
        Program { ops }
    }
}

/// Remove style off/on pairs (e.g., SetBold(false), SetBold(true) → remove both).
/// This optimizes patterns where Text components auto-reset styles.
fn collapse_style_toggles(ops: Vec<Op>) -> Vec<Op> {
    if ops.is_empty() {
        return ops;
    }

    let mut result = Vec::with_capacity(ops.len());
    let mut i = 0;

    while i < ops.len() {
        // Check for off/on toggle pairs
        if i + 1 < ops.len() {
            let collapse = match (&ops[i], &ops[i + 1]) {
                (Op::SetBold(false), Op::SetBold(true)) => true,
                (Op::SetUnderline(false), Op::SetUnderline(true)) => true,
                (Op::SetUpperline(false), Op::SetUpperline(true)) => true,
                (Op::SetInvert(false), Op::SetInvert(true)) => true,
                (Op::SetUpsideDown(false), Op::SetUpsideDown(true)) => true,
                (Op::SetReduced(false), Op::SetReduced(true)) => true,
                (Op::SetDoubleWidth(false), Op::SetDoubleWidth(true)) => true,
                (Op::SetDoubleHeight(false), Op::SetDoubleHeight(true)) => true,
                (Op::SetSmoothing(false), Op::SetSmoothing(true)) => true,
                // Also collapse size resets followed by same size
                (
                    Op::SetSize {
                        height: 0,
                        width: 0,
                    },
                    Op::SetSize {
                        height: h,
                        width: w,
                    },
                ) if *h > 0 || *w > 0 => {
                    // Keep the second SetSize, skip the reset
                    result.push(ops[i + 1].clone());
                    i += 2;
                    continue;
                }
                _ => false,
            };

            if collapse {
                i += 2; // Skip both ops
                continue;
            }
        }

        result.push(ops[i].clone());
        i += 1;
    }

    result
}

/// Remove duplicate Init ops, keeping only the first one.
fn remove_redundant_init(ops: Vec<Op>) -> Vec<Op> {
    let mut seen_init = false;
    ops.into_iter()
        .filter(|op| {
            if matches!(op, Op::Init) {
                if seen_init {
                    return false;
                }
                seen_init = true;
            }
            true
        })
        .collect()
}

/// Remove style changes that don't change the current state.
fn remove_redundant_styles(ops: Vec<Op>) -> Vec<Op> {
    let mut result = Vec::with_capacity(ops.len());
    let mut state = StyleState::default();

    for op in ops {
        match &op {
            Op::Init | Op::ResetStyle => {
                state = StyleState::default();
                result.push(op);
            }

            Op::SetAlign(a) => {
                if *a != state.alignment {
                    state.alignment = *a;
                    result.push(op);
                }
            }
            Op::SetFont(f) => {
                if *f != state.font {
                    state.font = *f;
                    result.push(op);
                }
            }
            Op::SetBold(b) => {
                if *b != state.bold {
                    state.bold = *b;
                    result.push(op);
                }
            }
            Op::SetUnderline(u) => {
                if *u != state.underline {
                    state.underline = *u;
                    result.push(op);
                }
            }
            Op::SetInvert(i) => {
                if *i != state.invert {
                    state.invert = *i;
                    result.push(op);
                }
            }
            Op::SetSize { height, width } => {
                if *height != state.height_mult || *width != state.width_mult {
                    state.height_mult = *height;
                    state.width_mult = *width;
                    result.push(op);
                }
            }

            // Non-style ops pass through unchanged
            _ => result.push(op),
        }
    }

    result
}

/// Merge consecutive Text ops into a single op.
fn merge_adjacent_text(ops: Vec<Op>) -> Vec<Op> {
    let mut result = Vec::with_capacity(ops.len());
    let mut pending_text: Option<String> = None;

    for op in ops {
        match op {
            Op::Text(s) => {
                if let Some(ref mut pending) = pending_text {
                    pending.push_str(&s);
                } else {
                    pending_text = Some(s);
                }
            }
            other => {
                // Flush any pending text
                if let Some(text) = pending_text.take() {
                    result.push(Op::Text(text));
                }
                result.push(other);
            }
        }
    }

    // Flush final pending text
    if let Some(text) = pending_text {
        result.push(Op::Text(text));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::text::Alignment;

    #[test]
    fn test_remove_redundant_init() {
        let ops = vec![
            Op::Init,
            Op::Text("a".into()),
            Op::Init,
            Op::Text("b".into()),
        ];
        let result = remove_redundant_init(ops);
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], Op::Init));
        assert!(matches!(result[1], Op::Text(_)));
        assert!(matches!(result[2], Op::Text(_)));
    }

    #[test]
    fn test_remove_redundant_styles_bold() {
        let ops = vec![
            Op::Init,
            Op::SetBold(true),
            Op::SetBold(true), // Redundant
            Op::Text("bold".into()),
            Op::SetBold(false),
            Op::SetBold(false), // Redundant
        ];
        let result = remove_redundant_styles(ops);
        assert_eq!(result.len(), 4); // Init, SetBold(true), Text, SetBold(false)
    }

    #[test]
    fn test_remove_redundant_styles_alignment() {
        let ops = vec![
            Op::Init,
            Op::SetAlign(Alignment::Center),
            Op::SetAlign(Alignment::Center), // Redundant
            Op::Text("centered".into()),
            Op::SetAlign(Alignment::Left), // Not redundant (different)
            Op::SetAlign(Alignment::Left), // Redundant
        ];
        let result = remove_redundant_styles(ops);
        assert_eq!(result.len(), 4); // Init, SetAlign(Center), Text, SetAlign(Left)
    }

    #[test]
    fn test_remove_redundant_styles_after_init() {
        let ops = vec![
            Op::Init,
            Op::SetAlign(Alignment::Left), // Redundant (default after init)
            Op::SetBold(false),            // Redundant (default)
            Op::Text("text".into()),
        ];
        let result = remove_redundant_styles(ops);
        assert_eq!(result.len(), 2); // Init, Text
    }

    #[test]
    fn test_remove_redundant_styles_after_reset() {
        let ops = vec![
            Op::Init,
            Op::SetBold(true),
            Op::Text("bold".into()),
            Op::ResetStyle,
            Op::SetBold(false), // Redundant (default after reset)
            Op::Text("normal".into()),
        ];
        let result = remove_redundant_styles(ops);
        assert_eq!(result.len(), 5); // Init, SetBold(true), Text, ResetStyle, Text
    }

    #[test]
    fn test_merge_adjacent_text() {
        let ops = vec![
            Op::Text("Hello".into()),
            Op::Text(" ".into()),
            Op::Text("World".into()),
        ];
        let result = merge_adjacent_text(ops);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Op::Text("Hello World".into()));
    }

    #[test]
    fn test_merge_text_interrupted_by_newline() {
        let ops = vec![
            Op::Text("Line 1".into()),
            Op::Newline,
            Op::Text("Line 2".into()),
        ];
        let result = merge_adjacent_text(ops);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Op::Text("Line 1".into()));
        assert_eq!(result[1], Op::Newline);
        assert_eq!(result[2], Op::Text("Line 2".into()));
    }

    #[test]
    fn test_merge_text_interrupted_by_style() {
        let ops = vec![
            Op::Text("normal".into()),
            Op::SetBold(true),
            Op::Text("bold".into()),
        ];
        let result = merge_adjacent_text(ops);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Op::Text("normal".into()));
        assert!(matches!(result[1], Op::SetBold(true)));
        assert_eq!(result[2], Op::Text("bold".into()));
    }

    #[test]
    fn test_full_optimization() {
        let ops = vec![
            Op::Init,
            Op::Init,           // Redundant
            Op::SetBold(false), // Redundant (default)
            Op::SetAlign(Alignment::Center),
            Op::SetAlign(Alignment::Center), // Redundant
            Op::Text("Hello".into()),
            Op::Text(" World".into()), // Merge with previous
            Op::Newline,
            Op::SetBold(true),
            Op::SetBold(true), // Redundant
            Op::Text("Bold".into()),
        ];

        let program = Program { ops };
        let optimized = program.optimize();

        // Expected: Init, SetAlign(Center), Text("Hello World"), Newline, SetBold(true), Text("Bold")
        assert_eq!(optimized.len(), 6);
        assert_eq!(optimized.ops[0], Op::Init);
        assert_eq!(optimized.ops[1], Op::SetAlign(Alignment::Center));
        assert_eq!(optimized.ops[2], Op::Text("Hello World".into()));
        assert_eq!(optimized.ops[3], Op::Newline);
        assert_eq!(optimized.ops[4], Op::SetBold(true));
        assert_eq!(optimized.ops[5], Op::Text("Bold".into()));
    }

    #[test]
    fn test_size_optimization() {
        let ops = vec![
            Op::Init,
            Op::SetSize {
                height: 0,
                width: 0,
            }, // Redundant (default)
            Op::SetSize {
                height: 1,
                width: 1,
            },
            Op::SetSize {
                height: 1,
                width: 1,
            }, // Redundant
            Op::Text("big".into()),
        ];
        let result = remove_redundant_styles(ops);
        assert_eq!(result.len(), 3); // Init, SetSize(1,1), Text
    }
}
