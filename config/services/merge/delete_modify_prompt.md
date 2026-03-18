File: {{.Merge.file_path}}

Agent A ({{.Merge.step_a_name}}: "{{.Merge.step_a_description}}") DELETED this file.

Agent B ({{.Merge.step_b_name}}: "{{.Merge.step_b_description}}") MODIFIED this file:
--- Changes ---
{{.Merge.diff_summary}}

Should this file be kept (with Agent B's modifications) or deleted?
Respond with exactly one word: KEEP or DELETE.
