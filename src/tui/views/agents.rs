//! Agents view showing agent pool status by tier

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::types::{Agent, AgentStatus, AgentTier};

/// View displaying agent pool status
pub struct AgentsView {
    pub agents: Vec<Agent>,
}

impl Default for AgentsView {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
        }
    }
}

impl AgentsView {
    pub fn new(agents: Vec<Agent>) -> Self {
        Self { agents }
    }

    fn agents_by_tier(&self, tier: AgentTier) -> Vec<&Agent> {
        self.agents.iter().filter(|a| a.tier == tier).collect()
    }

    fn status_display(status: &AgentStatus) -> (&'static str, Color) {
        match status {
            AgentStatus::Idle => ("Idle", Color::Gray),
            AgentStatus::Working => ("Working", Color::Green),
            AgentStatus::WaitingForContext => ("Waiting", Color::Yellow),
            AgentStatus::WaitingForApproval => ("Approval", Color::Cyan),
        }
    }

    fn render_tier(&self, tier: AgentTier, y: &mut u16, area: Rect, buf: &mut Buffer) {
        if *y >= area.bottom().saturating_sub(2) {
            return;
        }

        let agents = self.agents_by_tier(tier);
        let active = agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count();

        // Tier header
        let tier_name = match tier {
            AgentTier::Orchestrator => "Orchestrators",
            AgentTier::Worker => "Workers",
            AgentTier::Utility => "Utilities",
        };
        let header = format!("## {} ({}/{} active)", tier_name, active, agents.len());
        buf.set_string(
            area.x,
            *y,
            &header,
            Style::default().add_modifier(Modifier::BOLD),
        );
        *y += 2;

        if agents.is_empty() {
            buf.set_string(
                area.x + 2,
                *y,
                "No agents in pool",
                Style::default().fg(Color::DarkGray),
            );
            *y += 2;
            return;
        }

        for agent in agents {
            if *y >= area.bottom().saturating_sub(1) {
                break;
            }

            let (status_text, status_color) = Self::status_display(&agent.status);
            let task_info = agent
                .current_task
                .as_ref()
                .map(|t| format!(" -> {}", &t.0.to_string()[..8]))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{} ", agent.persona.name),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("[{}]", status_text),
                    Style::default().fg(status_color),
                ),
                Span::styled(task_info, Style::default().fg(Color::DarkGray)),
            ]);

            buf.set_line(area.x, *y, &line, area.width);
            *y += 1;
        }
        *y += 1; // Gap between tiers
    }
}

impl Widget for AgentsView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.agents.is_empty() {
            let msg = "No agents in pool. Agents are spawned when work begins.";
            buf.set_string(
                area.x + 2,
                area.y + 2,
                msg,
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let mut y = area.y + 1;

        // Summary line
        let total = self.agents.len();
        let working = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count();
        let summary = format!("Agent Pool: {}/{} active", working, total);
        buf.set_string(
            area.x,
            y,
            &summary,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        y += 2;

        self.render_tier(AgentTier::Orchestrator, &mut y, area, buf);
        self.render_tier(AgentTier::Worker, &mut y, area, buf);
        self.render_tier(AgentTier::Utility, &mut y, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentPersona, CommunicationStyle, ModelConfig};

    fn mock_agent(name: &str, tier: AgentTier, status: AgentStatus) -> Agent {
        Agent {
            id: crate::types::AgentId::new(),
            tier,
            persona: AgentPersona {
                name: name.to_string(),
                system_prompt: "Test agent".to_string(),
                style: CommunicationStyle::Casual,
            },
            model_config: ModelConfig::default(),
            current_task: None,
            status,
        }
    }

    #[test]
    fn agents_view_default_empty() {
        let view = AgentsView::default();
        assert!(view.agents.is_empty());
    }

    #[test]
    fn agents_view_with_agents() {
        let agents = vec![
            mock_agent("Orchestrator 1", AgentTier::Orchestrator, AgentStatus::Idle),
            mock_agent("Worker 1", AgentTier::Worker, AgentStatus::Working),
        ];
        let view = AgentsView::new(agents);
        assert_eq!(view.agents.len(), 2);
    }

    #[test]
    fn agents_by_tier_filters_correctly() {
        let agents = vec![
            mock_agent("Orch 1", AgentTier::Orchestrator, AgentStatus::Idle),
            mock_agent("Worker 1", AgentTier::Worker, AgentStatus::Working),
            mock_agent("Worker 2", AgentTier::Worker, AgentStatus::Idle),
            mock_agent("Utility 1", AgentTier::Utility, AgentStatus::Idle),
        ];
        let view = AgentsView::new(agents);

        assert_eq!(view.agents_by_tier(AgentTier::Orchestrator).len(), 1);
        assert_eq!(view.agents_by_tier(AgentTier::Worker).len(), 2);
        assert_eq!(view.agents_by_tier(AgentTier::Utility).len(), 1);
    }

    #[test]
    fn status_display_returns_correct_colors() {
        let (idle_text, idle_color) = AgentsView::status_display(&AgentStatus::Idle);
        let (working_text, working_color) = AgentsView::status_display(&AgentStatus::Working);

        assert_eq!(idle_text, "Idle");
        assert_eq!(idle_color, Color::Gray);
        assert_eq!(working_text, "Working");
        assert_eq!(working_color, Color::Green);
    }
}
