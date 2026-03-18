File: {{.Merge.file_path}}
Type: {{.Merge.file_type}}

Two agents independently created this file. Merge them into one coherent file.

--- AGENT A ({{.Merge.step_a_name}}: "{{.Merge.step_a_description}}") ---
{{.Merge.content_a}}

--- AGENT B ({{.Merge.step_b_name}}: "{{.Merge.step_b_description}}") ---
{{.Merge.content_b}}

Produce the complete merged file.
