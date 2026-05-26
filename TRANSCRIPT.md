design a workflow where a writer, illustrator, and musician collaborate to produce a children's book with an audiobook.

mkdir -p nodes && cat > topology.json << 'EOF'
{
  "nodes": {
    "brainstorm_idea": { "depends_on": [] },
    "write_manuscript": { "depends_on": ["brainstorm_idea"] },
    "create_illustrations": { "depends_on": ["write_manuscript"] },
    "compose_music": { "depends_on": ["write_manuscript"] },
    "record_narration": { "depends_on": ["write_manuscript"] },
    "produce_audiobook": { "depends_on": ["record_narration", "compose_music"] },
    "layout_book": { "depends_on": ["write_manuscript", "create_illustrations"] },
    "final_review": { "depends_on": ["produce_audiobook", "layout_book"] }
  }
}
EOF
&& cat > nodes/brainstorm_idea.md << 'EOF'
Brainstorm an engaging concept for a children's picture book.
EOF
&& cat > nodes/write_manuscript.md << 'EOF'
Write the full manuscript for the children's book.
EOF
&& cat > nodes/create_illustrations.md << 'EOF'
Create vibrant illustrations for the children's story.
EOF
&& cat > nodes/compose_music.md << 'EOF'
Compose original music and songs for the children's story.
EOF
&& cat > nodes/record_narration.md << 'EOF'
Record an expressive narration of the manuscript.
EOF
&& cat > nodes/produce_audiobook.md << 'EOF'
Mix the narration and music into a complete audiobook.
EOF
&& cat > nodes/layout_book.md << 'EOF'
Layout the picture book combining the text and illustrations.
EOF
&& cat > nodes/final_review.md << 'EOF'
Review the book and audiobook together. Make final improvements.
EOF
{"exit_code":0,"stderr":"","stdout":"total 16\ndrwxr-xr-x 3 root root 4096 Apr 11 20:10 .\ndrwxr-xr-x 3 root root 4096 Apr 11 20:10 ..\ndrwxr-xr-x 2 root root 4096 Apr 11 20:12 nodes\n-rw-r--r-- 1 root root  777 Apr 11 20:12 topology.json\n","success":true,"write_validation_errors":["nodes/final_review.md: nodes/final_review.md is empty","nodes/produce_audiobook.md: nodes/produce_audiobook.md is empty","nodes/layout_book.md: nodes/layout_book.md is empty","nodes/record_narration.md: nodes/record_narration.md is empty","nodes/compose_music.md: nodes/compose_music.md is empty"]}
cat topology.json
{"exit_code":0,"stderr":"","stdout":"{\n  \"nodes\": {\n    \"write_manuscript\": {\n      \"depends_on\": [\n        \"brainstorm_idea\"\n      ]\n    },\n    \"produce_audiobook\": {\n      \"depends_on\": [\n        \"compose_music\",\n        \"record_narration\"\n      ]\n    },\n    \"compose_music\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"record_narration\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"final_review\": {\n      \"depends_on\": [\n        \"layout_book\",\n        \"produce_audiobook\"\n      ]\n    },\n    \"create_illustrations\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"brainstorm_idea\": {\n      \"depends_on\": []\n    },\n    \"layout_book\": {\n      \"depends_on\": [\n        \"write_manuscript\",\n        \"create_illustrations\"\n      ]\n    }\n  }\n}","success":true,"write_validation_errors":["nodes/final_review.md: nodes/final_review.md is empty","nodes/produce_audiobook.md: nodes/produce_audiobook.md is empty","nodes/layout_book.md: nodes/layout_book.md is empty","nodes/record_narration.md: nodes/record_narration.md is empty","nodes/compose_music.md: nodes/compose_music.md is empty"]}
ls -la nodes/
{"exit_code":0,"stderr":"","stdout":"Brainstorm an engaging concept for a children's picture book.-----\nWrite the full manuscript for the children's book.-----\nCreate vibrant illustrations for the children's story.-----\n-----\n-----\n-----\n-----\n","success":true,"write_validation_errors":["nodes/final_review.md: nodes/final_review.md is empty","nodes/produce_audiobook.md: nodes/produce_audiobook.md is empty","nodes/layout_book.md: nodes/layout_book.md is empty","nodes/record_narration.md: nodes/record_narration.md is empty","nodes/compose_music.md: nodes/compose_music.md is empty"]}
"mkdir -p nodes && cat > nodes/brainstorm_idea.md << 'EOF'
Brainstorm an engaging concept for a children's picture book.
EOF
cat > nodes/write_manuscript.md << 'EOF'
Write the full manuscript for the children's book.
EOF
cat > nodes/create_illustrations.md << 'EOF'
Create vibrant illustrations for the children's story.
EOF
cat > nodes/compose_music.md << 'EOF'
Compose original music and songs for the children's story.
EOF
cat > nodes/record_narration.md << 'EOF'
Record an expressive narration of the manuscript.
EOF
cat > nodes/produce_audiobook.md << 'EOF'
Mix the narration and music into a complete audiobook.
EOF
cat > nodes/layout_book.md << 'EOF'
Layout the picture book combining the text and illustrations.
EOF
cat > nodes/final_review.md << 'EOF'
Review the book and audiobook together. Make final improvements.
EOF"
{"exit_code":0,"stderr":"","stdout":"","success":true}
mkdir -p nodes
{"exit_code":127,"stderr":"sh: 1: cat topology.json: not found\n","stdout":"","success":false}
"cat > nodes/brainstorm_idea.md << 'EOF'
Brainstorm an engaging concept for a children's picture book.
EOF"
{"exit_code":0,"stderr":"","stdout":"{\n  \"nodes\": {\n    \"write_manuscript\": {\n      \"depends_on\": [\n        \"brainstorm_idea\"\n      ]\n    },\n    \"produce_audiobook\": {\n      \"depends_on\": [\n        \"compose_music\",\n        \"record_narration\"\n      ]\n    },\n    \"compose_music\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"record_narration\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"final_review\": {\n      \"depends_on\": [\n        \"layout_book\",\n        \"produce_audiobook\"\n      ]\n    },\n    \"create_illustrations\": {\n      \"depends_on\": [\n        \"write_manuscript\"\n      ]\n    },\n    \"brainstorm_idea\": {\n      \"depends_on\": []\n    },\n    \"layout_book\": {\n      \"depends_on\": [\n        \"write_manuscript\",\n        \"create_illustrations\"\n      ]\n    }\n  }\n}","success":true}
cat > nodes/brainstorm_idea.md << 'EOF'
Brainstorm an engaging concept for a children's picture book.
EOF
{"exit_code":127,"stderr":"sh: 1: cat nodes/brainstorm_idea.md && echo '-----' && cat nodes/write_manuscript.md && echo '-----' && cat nodes/create_illustrations.md && echo '-----' && cat nodes/compose_music.md && echo '-----' && cat nodes/record_narration.md && echo '-----' && cat nodes/layout_book.md && echo '-----' && cat nodes/produce_audiobook.md && echo '-----' && cat nodes/final_review.md: not found\n","stdout":"","success":false}
"cat > nodes/write_manuscript.md << 'EOF'\nWrite the full manuscript for the children's book.\nEOF"
{"exit_code":0,"stderr":"","stdout":"Brainstorm an engaging concept for a children's picture book.-----\nWrite the full manuscript for the children's book.-----\nCreate vibrant illustrations for the children's story.-----\nCompose original music for the children's story.\n-----\nRecord narration for the children's book.\n-----\nLayout the children's book with text and illustrations.\n-----\nProduce the audiobook combining narration and music.\n-----\nReview and finalize the children's book and audiobook.\n","success":true}
cat > nodes/write_manuscript.md << 'EOF'
Write the full manuscript for the children's book.
EOF
{"exit_code":0,"stderr":"","stdout":"brainstorm_idea.md\ncompose_music.md\ncreate_illustrations.md\nfinal_review.md\nlayout_book.md\nproduce_audiobook.md\nrecord_narration.md\nwrite_manuscript.md\n","success":true}
"cat > nodes/create_illustrations.md << 'EOF'\nCreate vibrant illustrations for the children's story.\nEOF"
{"exit_code":127,"stderr":"sh: 1: cat > nodes/create_illustrations.md << 'EOF'\\nCreate vibrant illustrations for the children's story.\\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/layout_book.md: listed in topology.json but file does not exist","nodes/compose_music.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/create_illustrations.md: listed in topology.json but file does not exist","nodes/produce_audiobook.md: listed in topology.json but file does not exist"]}
"cat > nodes/create_illustrations.md << 'EOF'\nCreate vibrant illustrations for the children's story.\nEOF"
{"exit_code":127,"stderr":"sh: 1: cat > nodes/create_illustrations.md << 'EOF'\\nCreate vibrant illustrations for the children's story.\\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/layout_book.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/produce_audiobook.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist","nodes/compose_music.md: listed in topology.json but file does not exist","nodes/create_illustrations.md: listed in topology.json but file does not exist"]}
cat > nodes/create_illustrations.md << 'EOF'
Create vibrant illustrations for the children's story.
EOF
{"exit_code":0,"stderr":"","stdout":"","success":true,"write_validation_errors":["nodes/produce_audiobook.md: listed in topology.json but file does not exist","nodes/compose_music.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/layout_book.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist"]}
"cat > nodes/compose_music.md << 'EOF'\nCompose original music and songs for the children's story.\nEOF"
{"exit_code":127,"stderr":"sh: 1: cat > nodes/compose_music.md << 'EOF'\\nCompose original music and songs for the children's story.\\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/compose_music.md: listed in topology.json but file does not exist","nodes/layout_book.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/produce_audiobook.md: listed in topology.json but file does not exist"]}
"cat > nodes/compose_music.md << 'EOF'\nCompose original music and songs for the children's story.\nEOF"
{"exit_code":127,"stderr":"sh: 1: cat > nodes/compose_music.md << 'EOF'\\nCompose original music and songs for the children's story.\\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/produce_audiobook.md: listed in topology.json but file does not exist","nodes/layout_book.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/compose_music.md: listed in topology.json but file does not exist"]}
"cat > nodes/compose_music.md << 'EOF'\nCompose original music and songs for the children's story.\nEOF"
{"exit_code":127,"stderr":"sh: 1: cat > nodes/compose_music.md << 'EOF'\\nCompose original music and songs for the children's story.\\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/compose_music.md: listed in topology.json but file does not exist","nodes/layout_book.md: listed in topology.json but file does not exist","nodes/produce_audiobook.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist"]}
"cat > nodes/compose_music.md << 'EOF'
Compose original music and songs for the children's story.
EOF"
{"exit_code":127,"stderr":"sh: 3: cat > nodes/compose_music.md << 'EOF'\nCompose original music and songs for the children's story.\nEOF: not found\n","stdout":"","success":false,"write_validation_errors":["nodes/compose_music.md: listed in topology.json but file does not exist","nodes/layout_book.md: listed in topology.json but file does not exist","nodes/produce_audiobook.md: listed in topology.json but file does not exist","nodes/final_review.md: listed in topology.json but file does not exist","nodes/record_narration.md: listed in topology.json but file does not exist"]}
Go ahead and finish, you got interupted.

ls -la
cat topology.json
cat nodes/brainstorm_idea.md && echo "-----" && cat nodes/write_manuscript.md && echo "-----" && cat nodes/create_illustrations.md && echo "-----" && cat nodes/compose_music.md && echo "-----" && cat nodes/record_narration.md && echo "-----" && cat nodes/layout_book.md && echo "-----" && cat nodes/produce_audiobook.md && echo "-----" && cat nodes/final_review.md
cat > nodes/compose_music.md << 'EOF'
Compose original music for the children's story.
EOF
cat > nodes/record_narration.md << 'EOF'
Record narration for the children's book.
EOF
cat > nodes/layout_book.md << 'EOF'
Layout the children's book with text and illustrations.
EOF
cat > nodes/produce_audiobook.md << 'EOF'
Produce the audiobook combining narration and music.
EOF
cat > nodes/final_review.md << 'EOF'
Review and finalize the children's book and audiobook.
EOF
"cat topology.json"
cat topology.json
"cat nodes/brainstorm_idea.md && echo '-----' && cat nodes/write_manuscript.md && echo '-----' && cat nodes/create_illustrations.md && echo '-----' && cat nodes/compose_music.md && echo '-----' && cat nodes/record_narration.md && echo '-----' && cat nodes/layout_book.md && echo '-----' && cat nodes/produce_audiobook.md && echo '-----' && cat nodes/final_review.md"
cat nodes/brainstorm_idea.md && echo '-----' && cat nodes/write_manuscript.md && echo '-----' && cat nodes/create_illustrations.md && echo '-----' && cat nodes/compose_music.md && echo '-----' && cat nodes/record_narration.md && echo '-----' && cat nodes/layout_book.md && echo '-----' && cat nodes/produce_audiobook.md && echo '-----' && cat nodes/final_review.md
ls nodes
Workflow complete. The board now shows a clean collaborative story for the writer, illustrator, and musician:

▸
Brainstorm Idea → Write Manuscript

▸
From the manuscript, three parallel creative tracks:

◦
Create Illustrations (illustrator)

◦
Compose Music (musician)

◦
Record Narration (voice performer)

▸
Layout Book (manuscript + illustrations)

▸
Produce Audiobook (narration + music)

▸
Final Review (both the physical book and audiobook)

All node texts are short, self-contained, and read like sticky notes on a whiteboard. The topology matches exactly what was already on the canvas.

You can now trigger any node or refine further if you'd like to adjust the flow.