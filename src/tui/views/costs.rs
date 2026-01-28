//! Costs view showing cost breakdown by tier, model, and task

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::types::CostSummary;

/// View displaying cost breakdown
pub struct CostsView {
    pub summary: CostSummary,
}

impl Default for CostsView {
    fn default() -> Self {
        Self {
            summary: CostSummary::default(),
        }
    }
}

impl CostsView {
    pub fn new(summary: CostSummary) -> Self {
        Self { summary }
    }

    fn format_cost(cost: f64) -> String {
        if cost < 0.01 {
            format!("${:.4}", cost)
        } else if cost < 1.0 {
            format!("${:.3}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }

    fn render_bar(cost: f64, max_cost: f64, width: u16) -> String {
        if max_cost <= 0.0 {
            return " ".repeat(width as usize);
        }
        let pct = (cost / max_cost).min(1.0);
        let filled = ((width as f64) * pct).round() as usize;
        let empty = (width as usize).saturating_sub(filled);
        format!("{}{}", "|".repeat(filled), " ".repeat(empty))
    }
}

impl Widget for CostsView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y + 1;

        // Session total (prominent)
        let total_str = format!("Session Total: {}", Self::format_cost(self.summary.session_total));
        buf.set_string(
            area.x,
            y,
            &total_str,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        y += 2;

        // If no costs yet
        if self.summary.session_total == 0.0 {
            buf.set_string(
                area.x + 2,
                y,
                "No costs recorded yet. Costs appear as agents make LLM calls.",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        // By Tier
        if !self.summary.by_tier.is_empty() {
            buf.set_string(
                area.x,
                y,
                "## By Tier",
                Style::default().add_modifier(Modifier::BOLD),
            );
            y += 1;

            let tier_max = self
                .summary
                .by_tier
                .values()
                .copied()
                .fold(0.0f64, f64::max);

            let mut tiers: Vec<_> = self.summary.by_tier.iter().collect();
            tiers.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (tier, cost) in tiers {
                if y >= area.bottom().saturating_sub(2) {
                    break;
                }
                let bar = Self::render_bar(*cost, tier_max, 15);
                let line = format!(
                    "  {:12} [{}] {}",
                    tier,
                    bar,
                    Self::format_cost(*cost)
                );
                buf.set_string(area.x, y, &line, Style::default());
                y += 1;
            }
            y += 1;
        }

        // By Model
        if !self.summary.by_model.is_empty() && y < area.bottom().saturating_sub(4) {
            buf.set_string(
                area.x,
                y,
                "## By Model",
                Style::default().add_modifier(Modifier::BOLD),
            );
            y += 1;

            let model_max = self
                .summary
                .by_model
                .values()
                .copied()
                .fold(0.0f64, f64::max);

            let mut models: Vec<_> = self.summary.by_model.iter().collect();
            models.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (model, cost) in models.iter().take(5) {
                if y >= area.bottom().saturating_sub(2) {
                    break;
                }
                let bar = Self::render_bar(**cost, model_max, 15);
                let model_short = if model.len() > 15 {
                    format!("{}...", &model[..12])
                } else {
                    (*model).clone()
                };
                let line = format!(
                    "  {:15} [{}] {}",
                    model_short,
                    bar,
                    Self::format_cost(**cost)
                );
                buf.set_string(area.x, y, &line, Style::default());
                y += 1;
            }
            y += 1;
        }

        // Top tasks by cost
        if !self.summary.by_task.is_empty() && y < area.bottom().saturating_sub(4) {
            buf.set_string(
                area.x,
                y,
                "## Top Tasks by Cost",
                Style::default().add_modifier(Modifier::BOLD),
            );
            y += 1;

            let mut tasks: Vec<_> = self.summary.by_task.iter().collect();
            tasks.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (task_id, cost) in tasks.iter().take(5) {
                if y >= area.bottom().saturating_sub(1) {
                    break;
                }
                let task_short = if task_id.len() > 8 {
                    &task_id[..8]
                } else {
                    task_id
                };
                let line = format!("  {} - {}", task_short, Self::format_cost(**cost));
                buf.set_string(area.x, y, &line, Style::default());
                y += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn costs_view_default_empty() {
        let view = CostsView::default();
        assert_eq!(view.summary.session_total, 0.0);
    }

    #[test]
    fn costs_view_with_summary() {
        let mut by_tier = HashMap::new();
        by_tier.insert("Worker".to_string(), 0.05);
        by_tier.insert("Orchestrator".to_string(), 0.02);

        let summary = CostSummary {
            session_total: 0.07,
            by_tier,
            by_task: HashMap::new(),
            by_model: HashMap::new(),
        };
        let view = CostsView::new(summary);
        assert_eq!(view.summary.session_total, 0.07);
    }

    #[test]
    fn format_cost_small_values() {
        // Very small costs show 4 decimals
        assert_eq!(CostsView::format_cost(0.0012), "$0.0012");

        // Medium costs show 3 decimals
        assert_eq!(CostsView::format_cost(0.123), "$0.123");

        // Larger costs show 2 decimals
        assert_eq!(CostsView::format_cost(1.50), "$1.50");
    }

    #[test]
    fn render_bar_full() {
        let bar = CostsView::render_bar(1.0, 1.0, 10);
        assert_eq!(bar.len(), 10);
        assert!(bar.contains('|'));
    }

    #[test]
    fn render_bar_half() {
        let bar = CostsView::render_bar(0.5, 1.0, 10);
        assert_eq!(bar.len(), 10);
    }

    #[test]
    fn render_bar_empty_max() {
        let bar = CostsView::render_bar(0.5, 0.0, 10);
        assert_eq!(bar.len(), 10);
        assert!(!bar.contains('|')); // Should be all spaces
    }
}
