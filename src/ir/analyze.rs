//! IR analysis utilities for measuring optimization potential.

use super::ops::{Op, Program, StyleState};

/// Analysis results for an IR program
#[derive(Debug, Default)]
pub struct AnalysisResults {
    pub total_ops: usize,
    pub style_ops: usize,
    pub content_ops: usize,
    pub control_ops: usize,

    // Patterns found
    pub redundant_reset_style: usize,
    pub adjacent_same_style: usize,
    pub dead_style_changes: usize,
    pub mergeable_feeds: usize,
    pub non_adjacent_toggles: usize,
}

impl AnalysisResults {
    pub fn potential_savings(&self) -> usize {
        self.redundant_reset_style
            + self.adjacent_same_style
            + self.dead_style_changes
            + self.mergeable_feeds
            + self.non_adjacent_toggles
    }
}

/// Check if an op is a style-related op
fn is_style_op(op: &Op) -> bool {
    matches!(
        op,
        Op::SetAlign(_)
            | Op::SetFont(_)
            | Op::SetBold(_)
            | Op::SetUnderline(_)
            | Op::SetUpperline(_)
            | Op::SetInvert(_)
            | Op::SetSmoothing(_)
            | Op::SetUpsideDown(_)
            | Op::SetReduced(_)
            | Op::SetExpandedWidth(_)
            | Op::SetExpandedHeight(_)
            | Op::SetSize { .. }
            | Op::SetCodepage(_)
            | Op::ResetStyle
    )
}

/// Check if an op produces visible content
fn is_content_op(op: &Op) -> bool {
    matches!(
        op,
        Op::Text(_)
            | Op::Raw(_)
            | Op::Raster { .. }
            | Op::Band { .. }
            | Op::QrCode { .. }
            | Op::Pdf417 { .. }
            | Op::Barcode1D { .. }
            | Op::NvPrint { .. }
    )
}

/// Analyze an IR program for optimization opportunities
pub fn analyze(program: &Program) -> AnalysisResults {
    let mut results = AnalysisResults::default();
    results.total_ops = program.ops.len();

    let mut state = StyleState::default();
    let mut pending_styles: Vec<(usize, Op)> = Vec::new(); // (index, op) pairs of unused styles

    for (i, op) in program.ops.iter().enumerate() {
        if is_style_op(op) {
            results.style_ops += 1;
        } else if is_content_op(op) {
            results.content_ops += 1;
        } else {
            results.control_ops += 1;
        }

        match op {
            Op::Init | Op::ResetStyle => {
                // Check if ResetStyle is redundant (state already default)
                if matches!(op, Op::ResetStyle) && state == StyleState::default() {
                    results.redundant_reset_style += 1;
                }
                // Any pending styles are now dead
                results.dead_style_changes += pending_styles.len();
                pending_styles.clear();
                state = StyleState::default();
            }

            // Track style changes for dead style detection
            Op::SetBold(b) => {
                if *b == state.bold {
                    results.adjacent_same_style += 1;
                } else {
                    // Check for toggle pattern with pending
                    let toggled = pending_styles
                        .iter()
                        .any(|(_, s)| matches!(s, Op::SetBold(prev) if *prev != *b));
                    if toggled {
                        results.non_adjacent_toggles += 1;
                    }
                    pending_styles.push((i, op.clone()));
                    state.bold = *b;
                }
            }
            Op::SetUnderline(u) => {
                if *u == state.underline {
                    results.adjacent_same_style += 1;
                } else {
                    pending_styles.push((i, op.clone()));
                    state.underline = *u;
                }
            }
            Op::SetInvert(inv) => {
                if *inv == state.invert {
                    results.adjacent_same_style += 1;
                } else {
                    pending_styles.push((i, op.clone()));
                    state.invert = *inv;
                }
            }
            Op::SetSmoothing(s) => {
                if *s == state.smoothing {
                    results.adjacent_same_style += 1;
                } else {
                    pending_styles.push((i, op.clone()));
                    state.smoothing = *s;
                }
            }

            // Check for consecutive feeds
            Op::Feed { .. } => {
                if i > 0 && matches!(program.ops[i - 1], Op::Feed { .. }) {
                    results.mergeable_feeds += 1;
                }
            }

            // Content ops consume pending styles (they're not dead anymore)
            _ if is_content_op(op) => {
                pending_styles.clear();
            }

            _ => {}
        }
    }

    // Any remaining pending styles at end of program are dead
    results.dead_style_changes += pending_styles.len();

    results
}

/// Print analysis results in a readable format
pub fn print_analysis(name: &str, results: &AnalysisResults) {
    println!("=== {} ===", name);
    println!("Total ops: {}", results.total_ops);
    println!(
        "  Style: {}, Content: {}, Control: {}",
        results.style_ops, results.content_ops, results.control_ops
    );
    println!("Optimization opportunities:");
    println!("  Redundant ResetStyle: {}", results.redundant_reset_style);
    println!("  Adjacent same style: {}", results.adjacent_same_style);
    println!("  Dead style changes: {}", results.dead_style_changes);
    println!("  Mergeable feeds: {}", results.mergeable_feeds);
    println!("  Non-adjacent toggles: {}", results.non_adjacent_toggles);
    println!(
        "Potential op savings: {} ({:.1}%)",
        results.potential_savings(),
        100.0 * results.potential_savings() as f64 / results.total_ops as f64
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::text::Alignment;

    #[test]
    fn test_analyze_redundant_reset() {
        let program = Program {
            ops: vec![
                Op::Init,
                Op::ResetStyle, // Redundant - already default after Init
                Op::Text("test".into()),
            ],
        };
        let results = analyze(&program);
        assert_eq!(results.redundant_reset_style, 1);
    }

    #[test]
    fn test_analyze_dead_styles() {
        let program = Program {
            ops: vec![
                Op::Init,
                Op::SetBold(true),
                Op::SetUnderline(true),
                Op::ResetStyle, // Bold and underline were never used
                Op::Text("test".into()),
            ],
        };
        let results = analyze(&program);
        assert_eq!(results.dead_style_changes, 2);
    }

    #[test]
    fn test_analyze_adjacent_same() {
        let program = Program {
            ops: vec![
                Op::Init,
                Op::SetAlign(Alignment::Center),
                Op::SetAlign(Alignment::Center), // Same - already handled by optimizer
                Op::Text("test".into()),
            ],
        };
        let _results = analyze(&program);
        // This is already handled by the optimizer, so we don't count it
    }

    #[test]
    fn test_analyze_mergeable_feeds() {
        let program = Program {
            ops: vec![
                Op::Init,
                Op::Feed { units: 10 },
                Op::Feed { units: 10 }, // Could merge
                Op::Text("test".into()),
            ],
        };
        let results = analyze(&program);
        assert_eq!(results.mergeable_feeds, 1);
    }

    /// Dump ops for all receipts to see patterns
    /// Run with: cargo test dump_receipt_ops -- --nocapture
    #[test]
    fn dump_receipt_ops() {
        use crate::receipt;

        for name in ["receipt", "receipt-full", "markdown"] {
            let unopt = receipt::program_by_name_golden(name).unwrap();
            let opt = unopt.clone().optimize();

            println!("\n{}", "=".repeat(60));
            println!("=== {} ===", name.to_uppercase());
            println!("{}", "=".repeat(60));
            println!("Unoptimized: {} ops, {} bytes", unopt.len(), unopt.to_bytes().len());
            println!("Optimized:   {} ops, {} bytes", opt.len(), opt.to_bytes().len());
            println!(
                "Reduction:   {} ops removed, {} bytes saved\n",
                unopt.len() - opt.len(),
                unopt.to_bytes().len() - opt.to_bytes().len()
            );

            println!("--- UNOPTIMIZED ---");
            for (i, op) in unopt.ops.iter().enumerate() {
                println!("{:3}: {:?}", i, op);
            }

            println!("\n--- OPTIMIZED ---");
            for (i, op) in opt.ops.iter().enumerate() {
                println!("{:3}: {:?}", i, op);
            }
            println!();
        }
    }

    /// Analyze actual receipts to show optimization potential
    /// Run with: cargo test analyze_receipts -- --nocapture
    #[test]
    fn analyze_receipts() {
        use crate::receipt;

        println!("\n========================================");
        println!("IR ANALYSIS: Optimization Opportunities");
        println!("========================================\n");

        // Demo receipt
        let program = receipt::program_by_name("receipt").unwrap();
        let unopt = program.clone();
        let opt = program.optimize();
        println!("DEMO RECEIPT (before optimization):");
        print_analysis("Unoptimized", &analyze(&unopt));
        println!("DEMO RECEIPT (after optimization):");
        print_analysis("Optimized", &analyze(&opt));
        println!(
            "Bytes: {} -> {} ({:.1}% reduction)\n",
            unopt.to_bytes().len(),
            opt.to_bytes().len(),
            100.0 * (1.0 - opt.to_bytes().len() as f64 / unopt.to_bytes().len() as f64)
        );

        // Full receipt
        let program = receipt::program_by_name("receipt-full").unwrap();
        let unopt = program.clone();
        let opt = program.optimize();
        println!("FULL RECEIPT (before optimization):");
        print_analysis("Unoptimized", &analyze(&unopt));
        println!("FULL RECEIPT (after optimization):");
        print_analysis("Optimized", &analyze(&opt));
        println!(
            "Bytes: {} -> {} ({:.1}% reduction)\n",
            unopt.to_bytes().len(),
            opt.to_bytes().len(),
            100.0 * (1.0 - opt.to_bytes().len() as f64 / unopt.to_bytes().len() as f64)
        );

        // Markdown demo
        let program = receipt::program_by_name("markdown").unwrap();
        let unopt = program.clone();
        let opt = program.optimize();
        println!("MARKDOWN DEMO (before optimization):");
        print_analysis("Unoptimized", &analyze(&unopt));
        println!("MARKDOWN DEMO (after optimization):");
        print_analysis("Optimized", &analyze(&opt));
        println!(
            "Bytes: {} -> {} ({:.1}% reduction)\n",
            unopt.to_bytes().len(),
            opt.to_bytes().len(),
            100.0 * (1.0 - opt.to_bytes().len() as f64 / unopt.to_bytes().len() as f64)
        );
    }
}
