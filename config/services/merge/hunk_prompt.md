File: {{.Merge.file_path}} (lines {{.Merge.line_range}})
Type: {{.Merge.file_type}}

{{.Merge.context_block}}

--- BASE (before either agent) ---
{{.Merge.base_hunk}}

--- AGENT A ({{.Merge.step_a_name}}: "{{.Merge.step_a_description}}") ---
{{.Merge.version_a_hunk}}

--- AGENT B ({{.Merge.step_b_name}}: "{{.Merge.step_b_description}}") ---
{{.Merge.version_b_hunk}}

Merge both agents' changes for this region.
