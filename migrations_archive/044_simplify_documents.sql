-- Step 3.5: Simplify documents
-- Make summary, doc_type, ref_tag, tags nullable

ALTER TABLE documents ALTER COLUMN summary DROP NOT NULL;
ALTER TABLE documents ALTER COLUMN doc_type DROP NOT NULL;
ALTER TABLE documents ALTER COLUMN ref_tag DROP NOT NULL;
ALTER TABLE documents ALTER COLUMN tags DROP NOT NULL;
